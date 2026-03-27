pub mod helper;
mod logger;

use fork::{fork, Fork};
use futures::executor::block_on;
use futures::TryStreamExt;
use helper::{Config, TimestampActions};
use logger::Logger;
use netns_rs::NetNs;
use network_time_simulator::SIZE_BUFFER;
use network_time_simulator::{deserialize_rust, serialize_rust, Buffer, Message};
use nix::unistd::execve;
use posixmq::PosixMq;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rtnetlink::new_connection;
use std::collections::{BTreeMap, HashMap};
use std::env;
#[allow(unused)]
use std::ffi::{CStr, CString};
use std::io::Result;
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::{Command, Stdio};

/// Represents the current state of a follower process.
#[derive(Copy, Clone, PartialEq, Debug)]
enum State {
    /// The process is running or scheduled to run.
    Running,
    /// The process is blocked on a syscall (e.g., waiting for time or packet).
    Blocked,
    /// The process has terminated.
    Finished,
}

#[allow(unconditional_panic)]
const fn illegal_null_in_string() {
    [][0]
}

#[doc(hidden)]
pub const fn validate_cstr_contents(bytes: &[u8]) {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\0' {
            illegal_null_in_string();
        }
        i += 1;
    }
}

const LIB_NAME: &str = "./syscalls.so";
const DOCKER_LIB_NAME: &str = "/exe/syscalls.so";
const LOG_DIR: &str = "logs/";

/// Opens one queue on which the follower will send messages to the leader and
/// one queue on which the follower will receive messages from the leader
/// @arg id: the id of the follower, strictly positive
/// @arg nb: the size of the queue (use the number of followers)
/// @return: on success return a pair of posix queues (follower_receiving_queue, follower_sending_queue)
fn follower_init_queues(id: u8, nb: usize, qname: &String) -> Result<(PosixMq, PosixMq)> {
    // TODO: no more use
    // TODO: (for later) find a way to use this to avoid passing through the env
    let qo = posixmq::OpenOptions::writeonly() //the follower will send messages to the
        .max_msg_len(SIZE_BUFFER) //leader on this queue
        .capacity(nb)
        .create()
        .open(&format!("{}_{}", qname, 0))
        .expect(&format!("failed to open queue qo for a follower: {}", id).to_string());
    qo.set_cloexec(false)?;
    /*let qo;
    unsafe {
        let c_str = CString::new(format!("{}_{}", qname, 0)).unwrap();
        println!("c_str: {:?}", c_str);
        qo = libc::mq_open(c_str.as_ptr() as *const i8, libc::O_WRONLY);
        println!("qo: {:?}", qo);
        libc::perror(c_str.as_ptr() as *const i8);
    };*/

    let qi = posixmq::OpenOptions::readonly() //the follower will receive the messages of
        .max_msg_len(SIZE_BUFFER) //the leader on this queue
        .capacity(nb)
        .create()
        .open(&format!("{}_{}", qname, id))
        .expect(&format!("failed to open queue qi for a follower: {}", id).to_string());
    qi.set_cloexec(false)?;
    return Ok((qi, qo));
}

/// The core simulator structure that manages the entire simulation life cycle.
#[derive(Debug)]
pub struct Simulation {
    /// Configuration for the simulation.
    cfg: Config,
    /// Current state of each follower process.
    states: Vec<State>,
    /// Future events scheduled at specific timestamps.
    events: BTreeMap<u64, TimestampActions>,
    /// Posix Message Queues for IPC (index 0 is for leader, 1..nb_follower are for followers).
    qs: Vec<PosixMq>,
    /// Random seed counters per follower for deterministic random generation.
    counters: HashMap<usize, (u64, u64)>, //val = (counter,seed)
    /// Logger for simulation events.
    logs: Logger,
    /// Docker container PIDs by node id when running in Docker mode.
    docker_pids: Option<HashMap<u8, i32>>,
}

impl Drop for Simulation {
    /// Cleanup logic: removes network namespaces and unlinks message queues.
    fn drop(&mut self) {
        // TODO: call this when ctrl+C is hit
        if self.cfg.mode == "docker" {
            // Stop containers (autoremoval is enabled on run). TODO: make cleanup policy configurable.
            if let Some(pids) = &self.docker_pids {
                for (id, _pid) in pids.iter() {
                    let name = format!("jade_node_{}", id);
                    let mut cmd_stop = Command::new("/usr/bin/env");
                    cmd_stop.arg("docker").arg("stop").arg("-t").arg("0").arg(name);
                    if let Ok(val) = env::var("DOCKER_HOST") {
                        cmd_stop.env("DOCKER_HOST", val);
                    }
                    let _ = cmd_stop.status();
                }
            }
        } else {
            //delete the namespaces
            for node in self.cfg.topo.grf.nodes.iter() {
                if let Ok(ns) = NetNs::get(node.id.to_string()) {
                    if let Err(_err) = ns.remove() {
                        eprintln!("Failed to remove namespace {}", node.id);
                    }
                } else {
                    eprintln!("Namespace {} doesn't exist", node.id);
                }
            }
        }
        let _ = self.cfg.unlink_queues();
    }
}

impl Simulation {
    fn new(cfg: Config) -> Self {
        let logs = Logger::new(
            &cfg.log_file,
            cfg.log_level.contains(&"trace".to_string()),
            cfg.log_level.contains(&"debug".to_string()),
            cfg.log_level.contains(&"warn".to_string()),
            cfg.log_level.contains(&"error".to_string()),
            cfg.log_level.contains(&"info".to_string()),
            cfg.log_level.contains(&"message".to_string()),
        );
        let nb_f = cfg.nb_follower;
        let _ = cfg.unlink_queues();
        let btm = BTreeMap::new();

        let mut cfg = cfg;
        let docker_map: Option<HashMap<u8, i32>> = None;
        if cfg.mode != "docker" {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            cfg = rt
                .block_on(Self::create_namespaces(cfg, &logs))
                .expect("Failed to create namespaces");
            rt.shutdown_background();
        }
        let mut c = HashMap::with_capacity(nb_f);
        for i in 0..nb_f {
            c.insert(i + 1, (1, 0));
        }

        Self {
            cfg,
            states: vec![State::Running; nb_f],
            events: btm,
            qs: Vec::with_capacity(nb_f + 1),
            counters: c,
            logs: logs,
            docker_pids: docker_map,
        }
    }

    /// Creates one namespace for each node and one link for each edge. Attach the interface to
    /// the corresponding namespace and sets the interfaces up
    /// @arg cfg: the configuration of the current simulation
    /// @return: the configuration of the current simulation
    async fn create_namespaces(cfg: Config, logs: &Logger) -> Result<Config> {
        // Create the namespaces
        for node in cfg.topo.grf.nodes.iter() {
            if let Ok(_) = NetNs::get(node.id.to_string()) {
                eprintln!("Namespace {:?} already exists", node.id);
            } else {
                NetNs::new(node.id.to_string())
                    .expect(&format!("Failed to create namespace {:?}", node.id));
                //use node id as name
            }
        }

        // Adds the links between the namespaces
        for edge in cfg.topo.grf.edges.iter() {
            let ipv4_source: std::net::Ipv4Addr = cfg
                .topo
                .ip4_node
                .iter()
                .find_map(|(key, &val)| {
                    if val.0 == edge.source.try_into().unwrap() {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .expect("Source IPv4 address not found")
                .try_into()
                .unwrap();

            let ipv4_target: Ipv4Addr = cfg
                .topo
                .ip4_node
                .iter()
                .find_map(|(key, &val)| {
                    if val.0 == edge.target.try_into().unwrap() {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .expect("Target IPv4 address not found")
                .try_into()
                .unwrap();

            if cfg.topo.ip6_node.len() == 0 {
                if let Err(e) = block_on(Self::add_link(
                    edge.source.try_into().unwrap(),
                    edge.target.try_into().unwrap(),
                    ipv4_source,
                    Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
                    ipv4_target,
                    Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
                    logs,
                )) {
                    println!(
                        "Error {} : could not create link between {:?} and {:?}",
                        e, edge.source, edge.target
                    );
                }
            } else {
                let ipv6_source: Ipv6Addr = cfg
                    .topo
                    .ip6_node
                    .iter()
                    .find_map(|(key, &val)| {
                        if val.0 == edge.source.try_into().unwrap() {
                            Some(key.clone())
                        } else {
                            None
                        }
                    })
                    .expect("Source IPv6 address not found")
                    .try_into()
                    .unwrap();

                let ipv6_target: Ipv6Addr = cfg
                    .topo
                    .ip6_node
                    .iter()
                    .find_map(|(key, &val)| {
                        if val.0 == edge.target.try_into().unwrap() {
                            Some(key.clone())
                        } else {
                            None
                        }
                    })
                    .expect("Target IPv6 address not found")
                    .try_into()
                    .unwrap();

                if let Err(e) = block_on(Self::add_link(
                    edge.source.try_into().unwrap(),
                    edge.target.try_into().unwrap(),
                    ipv4_source,
                    ipv6_source,
                    ipv4_target,
                    ipv6_target,
                    logs,
                )) {
                    println!(
                        "Error {} : could not create link between {:?} and {:?}",
                        e, edge.source, edge.target
                    );
                }
            }
        }
        Ok(cfg)
    }

    #[allow(dead_code)]
    /// Adds a link between name space id_source_namespace and id_target_namespace,
    /// sets the interfaces up and adds the ipv4 and ipv6 provided to the interfaces
    /// @arg id_source_namespace: id of the source namespace of the link
    /// @arg id_target_namespace: id of the target namespace of the link
    /// @arg ipv4_source: ipv4 that must be attached to the source interface. 0.0.0.0 if no ipv4 must be attached
    /// @arg ipv6_source: ipv6 that must be attached to the source interface. 0:0:0:0:0:0:0:0 if no ipv6 must be attached
    /// @arg ipv4_target: ipv4 that must be attached to the target interface. 0.0.0.0 if no ipv4 must be attached
    /// @arg ipv6_target: ipv6 that must be attached to the target interface. 0:0:0:0:0:0:0:0 if no ipv6 must be attached
    /// @return: 0 on success
    async fn add_link(
        id_source_namespace: u8,
        id_target_namespace: u8,
        ipv4_source: Ipv4Addr,
        ipv6_source: Ipv6Addr,
        ipv4_target: Ipv4Addr,
        ipv6_target: Ipv6Addr,
        logs: &Logger,
    ) -> Result<i32> {
        let (connection, handle, _) = new_connection().unwrap();
        tokio::spawn(connection);

        // Interface names
        // Interface veth_sourceNode_targetNode is the interface of sourceNode that is connected to targetNode
        let name_interface1 = format!("veth_{}_{}", id_source_namespace, id_target_namespace);
        let name_interface2 = format!("veth_{}_{}", id_target_namespace, id_source_namespace);

        // Create veth pair
        let request = handle
            .link()
            .add()
            .veth(name_interface1.clone(), name_interface2.clone());
        if let Err(error) = request.execute().await.map_err(|e| format!("{}", e)) {
            logs.log("error", &format!("Could not create veth pair: {}", error));
            println!("Could not create veth pair: {}", error);
        }

        // Get interface index
        let links1: Vec<_> = handle
            .link()
            .get()
            .match_name(name_interface1.clone())
            .execute()
            .try_collect()
            .await
            .map_err(|e| {
                logs.log(
                    "error",
                    &format!("Failed to get interface information: {}", e),
                );
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to get interface information: {}", e),
                )
            })?;

        let link1 = links1.into_iter().next().ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("Interface {} not found", name_interface1),
            )
        })?;

        let links2: Vec<_> = handle
            .link()
            .get()
            .match_name(name_interface2.clone())
            .execute()
            .try_collect()
            .await
            .map_err(|e| {
                logs.log(
                    "error",
                    &format!("Failed to get interface information: {}", e),
                );
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to get interface information: {}", e),
                )
            })?;

        let link2 = links2.into_iter().next().ok_or_else(|| {
            logs.log("error", &format!("Interface {} not found", name_interface2));
            Error::new(
                ErrorKind::NotFound,
                format!("Interface {} not found", name_interface2),
            )
        })?;

        // Get namespace
        let source_ns = NetNs::get(id_source_namespace.to_string()).map_err(|e| {
            logs.log("error", &format!("Failed to get source namespace: {}", e));
            Error::new(
                ErrorKind::Other,
                format!("Failed to get source namespace: {}", e),
            )
        })?;

        let target_ns = NetNs::get(id_target_namespace.to_string()).map_err(|e| {
            logs.log("error", &format!("Failed to get target namespace: {}", e));

            Error::new(
                ErrorKind::Other,
                format!("Failed to get target namespace: {}", e),
            )
        })?;

        // Get the name space file descriptor
        let source_ns_fd = source_ns.file().as_raw_fd();
        let target_ns_fd = target_ns.file().as_raw_fd();

        // Put the interface in the right
        // Put the interface in the correspondig namespace and set it up
        let mut link_set_req1 = handle.link().set(link1.header.index);
        link_set_req1 = link_set_req1.setns_by_fd(source_ns_fd);
        link_set_req1
            .execute()
            .await
            .map_err(|e| {
                logs.log(
                    "error",
                    &format!(
                        "Failed to move {} to namespace {}: {}",
                        name_interface1, id_source_namespace, e
                    ),
                );

                Error::new(
                    ErrorKind::Other,
                    format!(
                        "Failed to move {} to namespace {}: {}",
                        name_interface1, id_source_namespace, e
                    ),
                )
            })
            .unwrap();

        let mut link_set_req2 = handle.link().set(link2.header.index);
        link_set_req2 = link_set_req2.setns_by_fd(target_ns_fd);
        link_set_req2
            .execute()
            .await
            .map_err(|e| {
                logs.log(
                    "error",
                    &format!(
                        "Failed to move {} to namespace {}: {}",
                        name_interface2, id_target_namespace, e
                    ),
                );
                Error::new(
                    ErrorKind::Other,
                    format!(
                        "Failed to move {} to namespace {}: {}",
                        name_interface2, id_target_namespace, e
                    ),
                )
            })
            .unwrap();

        // set both interfaces up
        Self::set_interface_up(
            id_source_namespace,
            id_target_namespace,
            ipv4_source,
            ipv6_source,
            &logs,
        )
        .await;
        Self::set_interface_up(
            id_target_namespace,
            id_source_namespace,
            ipv4_target,
            ipv6_target,
            &logs,
        )
        .await;

        Ok(0)
    }

    /// Deletes the link between namespace id_source_namespace and namespace id_target_namespace
    /// @arg id_source_namespace: id of the source namespace of the link
    /// @arg id_target_namespace: id of the target namespace of the link
    #[allow(dead_code)]
    async fn delete_link(id_source_namespace: u8, id_target_namespace: u8, logs: &Logger) {
        // Interface names
        // Interface veth_sourceNode_targetNode is the interface of sourceNode that is connected to targetNode
        let name_interface = format!("veth_{}_{}", id_source_namespace, id_target_namespace);

        let source_ns = match NetNs::get(id_source_namespace.to_string()) {
            Ok(ns) => ns,
            Err(e) => {
                logs.log(
                    "error",
                    &format!(
                        "Failed to get source namespace for ID {}: {}",
                        id_source_namespace, e
                    ),
                );
                eprintln!(
                    "Failed to get source namespace for ID {}: {}",
                    id_source_namespace, e
                );
                return;
            }
        };

        // Go to the right namespace
        if let Err(e) = source_ns.enter() {
            logs.log(
                "error",
                &format!(
                    "Error {:?} when entering the namespace {}",
                    e, id_source_namespace
                ),
            );
            println!(
                "Error {:?} when entering the namespace {}",
                e, id_source_namespace
            );
        }

        // Create new handle
        let (connection, handle, _) = new_connection().unwrap();
        tokio::spawn(connection);

        let mut links = handle
            .link()
            .get()
            .match_name(name_interface.clone())
            .execute();

        if let Some(link) = links.try_next().await.expect("Error while fetching links") {
            let link_idx = link.header.index;
            match handle.link().del(link_idx).execute().await {
                Ok(_) => {
                    println!("Successfully deleted the link {}", name_interface)
                }
                Err(e) => {
                    logs.log(
                        "info",
                        &format!("Failed to delete the link {}: {}", name_interface, e),
                    );
                    eprintln!("Failed to delete the link {}: {}", name_interface, e)
                }
            }
        } else {
            logs.log(
                "error",
                &format!("No link found with name {}", name_interface),
            );
            eprintln!("No link found with name {}", name_interface);
        }
    }

    /// Set the interface "veth_id_source_namespace_id_target_namespace" up
    /// and attributes the ipv4 and ipv6 to the interface
    /// @arg id_source_namespace: id of the namespace to which the interface is attached
    /// @arg id_target_namespace: id of the namespace to which the link goes
    /// @arg ipv4: ipv4 to attach to the interface 0.0.0.0 if no ipv6 must be attached
    //// @arg ipv6: ipv6 to attach to the interface 0:0:0:0:0:0:0:0 if no ipv6 must be attached
    #[allow(dead_code)]
    async fn set_interface_up(
        id_source_namespace: u8,
        id_target_namespace: u8,
        ipv4: Ipv4Addr,
        ipv6: Ipv6Addr,
        logs: &Logger,
    ) {

        // Interface names
        // Interface veth_sourceNode_targetNode is the interface of sourceNode that is connected to targetNode
        let name_interface = format!("veth_{}_{}", id_source_namespace, id_target_namespace);

        // Get namespace
        let source_ns = NetNs::get(id_source_namespace.to_string())
            .map_err(|e| {
                logs.log("error", &format!("Failed to get source namespace: {}", e));
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to get source namespace: {}", e),
                )
            })
            .unwrap();

        // Go to the right namespace
        if let Err(e) = source_ns.enter() {
            logs.log(
                "error",
                &format!(
                    "Error {:?} when entering the namespace {}",
                    e, id_source_namespace
                ),
            );
            println!(
                "Error {:?} when entering the namespace {}",
                e, id_source_namespace
            );
        }

        // Create new handle
        let (connection, handle, _) = new_connection().unwrap();
        tokio::spawn(connection);

        // Get interface idx
        let links1: Vec<_> = handle
            .link()
            .get()
            .match_name(name_interface.clone())
            .execute()
            .try_collect()
            .await
            .map_err(|e| {
                logs.log(
                    "error",
                    &format!("Failed to get interface information: {}", e),
                );
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to get interface information: {}", e),
                )
            })
            .unwrap();

        let link1 = links1
            .into_iter()
            .next()
            .ok_or_else(|| {
                logs.log("error", &format!("Interface {} not found", name_interface));
                Error::new(
                    ErrorKind::NotFound,
                    format!("Interface {} not found", name_interface),
                )
            })
            .unwrap();

        let interface_idx = link1.header.index;

        //set interface up
        if let Err(e) = handle.link().set(interface_idx).up().execute().await {
            logs.log(
                "error",
                &format!(
                    "Error {:?} could not set the interface {} up",
                    e, interface_idx
                ),
            );
            println!(
                "Error {:?} could not set the interface {} up",
                e, interface_idx
            );
        }

        // Is there an ipv4 to add
        if !ipv4.is_unspecified() {
            let mut has_ipv4 = false;

            // Retrieve the address associated with the interface
            let mut addresses = handle
                .address()
                .get()
                .set_link_index_filter(interface_idx)
                .execute();

            let addr_msg = addresses.try_next().await.unwrap();
            if addr_msg != None {
                if addr_msg.unwrap().attributes.contains(
                    &netlink_packet_route::address::AddressAttribute::Address(IpAddr::V4(ipv4)),
                ) {
                    has_ipv4 = true;
                }
            }
            if !has_ipv4 {
                let addr_req = handle
                    .address()
                    .add(link1.header.index, IpAddr::V4(ipv4), 24);
                if let Err(e) = addr_req.execute().await {
                    logs.log(
                        "error",
                        &format!(
                            "Error when adding the ipv4 to the interface {:?} : {:?}",
                            name_interface, e
                        ),
                    );
                    println!(
                        "Error when adding the ipv4 to the interface {:?} : {:?}",
                        name_interface, e
                    );
                }
            }
        }

        if !ipv6.is_unspecified() {
            let addr_req = handle.address().add(interface_idx, IpAddr::V6(ipv6), 64);
            if let Err(e) = addr_req.execute().await {
                logs.log(
                    "error",
                    &format!(
                        "Error when adding the ipv§ to the interface {:?} : {:?}",
                        name_interface, e
                    ),
                );
                println!(
                    "Error when add the ipv6 to the interface {:?} : {:?}",
                    name_interface, e
                );
            }
        }
    }

    /// Set the interface "veth_id_source_namespace_id_target_namespace" down
    /// @arg id_source_namespace: id of the namespace to which the interface is attached
    /// @arg id_target_namespace: id of the namespace to which the link goes
    #[allow(dead_code)]
    async fn set_interface_down(id_source_namespace: u8, id_target_namespace: u8, logs: Logger) {
        logs.log("trace", "entering simulation::set_interface_down");

        // Interface names
        // Interface veth_sourceNode_targetNode is the interface of sourceNode that is connected to targetNode
        let name_interface = format!("veth_{}_{}", id_source_namespace, id_target_namespace);

        // Get namespace
        let source_ns = NetNs::get(id_source_namespace.to_string())
            .map_err(|e| {
                logs.log("error", &format!("Failed to get source namespace: {}", e));
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to get source namespace: {}", e),
                )
            })
            .unwrap();

        // Go to the right namespace
        if let Err(e) = source_ns.enter() {
            logs.log(
                "error",
                &format!(
                    "Error {:?} when entering the namespace {}",
                    e, id_source_namespace
                ),
            );
            println!(
                "Error {:?} when entering the namespace {}",
                e, id_source_namespace
            );
        }

        // Create new handle
        let (connection, handle, _) = new_connection().unwrap();
        tokio::spawn(connection);

        // Get interface idx
        let links1: Vec<_> = handle
            .link()
            .get()
            .match_name(name_interface.clone())
            .execute()
            .try_collect()
            .await
            .map_err(|e| {
                logs.log(
                    "error",
                    &format!("Failed to get interface information: {}", e),
                );
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to get interface information: {}", e),
                )
            })
            .unwrap();

        let link1 = links1
            .into_iter()
            .next()
            .ok_or_else(|| {
                logs.log("error", &format!("Interface {} not found", name_interface));
                Error::new(
                    ErrorKind::NotFound,
                    format!("Interface {} not found", name_interface),
                )
            })
            .unwrap();

        let interface_idx = link1.header.index;

        // set interface down
        if let Err(e) = handle.link().set(interface_idx).down().execute().await {
            logs.log(
                "error",
                &format!(
                    "Error {:?} could not set the interface {} up",
                    e, interface_idx
                ),
            );
            println!(
                "Error {:?} could not set the interface {} up",
                e, interface_idx
            );
        }
    }

    /// Starts a follower with the queues to communicate toward the leader as file descriptor 3 and from
    /// the leader as file descriptor 4.
    /// @arg id: the id of the follower
    /// @return: the pid of the child on success
    fn run_follower(&mut self, id: u8) -> Result<i32> {
        if self.cfg.mode == "docker" {
            // In Docker mode, the container is already running with the actual process
            // (started in create_docker_containers). Just return the PID.
            // The process waits for JADE's signal before executing, which happens after this.
            if let Some(ref pids) = self.docker_pids {
                if let Some(&pid) = pids.get(&id) {
                    Ok(pid)
                } else {
                    self.logs.log("error", &format!("No PID found for node {}", id));
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        String::from("No PID found for container"),
                    ))
                }
            } else {
                self.logs.log("error", "Docker PIDs not initialized");
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    String::from("Docker PIDs not initialized"),
                ))
            }
        } else {
            // Find the executable config for this node ID (fallback to index-based if needed)
            let exe_cfg = self.cfg.exe.iter().find(|p| p.node_ids.contains(&id))
                .or_else(|| {
                    self.cfg.exe.get((id - 1) as usize)
                })
                .expect(&format!("Missing executable configuration for node ID {}", id));

            // get namespace
            let ns = NetNs::get(id.to_string()).unwrap();
            ns.run(|_| match fork() {
                Ok(Fork::Parent(child)) => Ok(child),
                Ok(Fork::Child) => {
                    let _ = follower_init_queues(id, self.cfg.nb_follower, &self.cfg._qname);
                    execve(
                        &exe_cfg.path,
                        &exe_cfg.args,
                        &exe_cfg.env)?;
                    Ok(0)
                }
                Err(_) => {
                    self.logs.log("error", "fork failed");
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        String::from("Fork failed"),
                    ))
                }
            })
                .unwrap()
        }
    }

    /// Initialize one queue on which the leader will wait for the messages from the followers and a
    /// queue by follower
    /// @arg nb: the number of follower
    /// @return: returns a Result
    fn leader_init_queues(&mut self) -> Result<()> {
        self.qs.push(
            posixmq::OpenOptions::readonly() //the leader will receive messages on this queue
                .max_msg_len(SIZE_BUFFER)
                .capacity(self.cfg.nb_follower)
                .create()
                .open(&format!("{}_{}", self.cfg._qname, 0))?,
        );

        for i in 0..self.cfg.nb_follower {
            self.qs.push(
                posixmq::OpenOptions::writeonly() //the leader will send messages on those queues
                    .max_msg_len(SIZE_BUFFER)
                    .capacity(self.cfg.nb_follower)
                    .create()
                    .open(&format!("{}_{}", self.cfg._qname, i + 1))?,
            );
        }
        Ok(())
    }

    /// Handles an IPC message from a follower and updates the simulation state.
    fn messages_handler(&mut self, message: &Buffer, current_time: u64) -> Result<()> {
        match deserialize_rust(*message) {
            Message::AddStep(id, t) => {

                if let Some(x) = self.events.get_mut(&t) {
                    x.add_process(id);
                } else {
                    self.events.insert(t, TimestampActions::new_process(id));
                }
            }
            Message::DelStep(id, t) => {
                if let Some(x) = self.events.get_mut(&t) {
                    x.del_process(id);
                }
            }
            Message::GetTime(id) => {
                //return head key of the BTreeMap

                let msg = serialize_rust(Message::WakeUp(current_time));
                self.qs[id as usize].send(2, &msg.buffer)?;
            }
            Message::GetRand(id, seed) => {

                // Random with seed.
                if seed == self.counters.get(&usize::from(id)).unwrap().1 {
                    self.counters
                        .entry(usize::from(id))
                        .and_modify(|x| x.0 += 1);
                } else {
                    self.counters.entry(usize::from(id)).and_modify(|x| {
                        x.0 = 1;
                        x.1 = seed;
                    });
                }
                let counter: u64 = self.counters.get(&usize::from(id)).unwrap().0;
                let computed_seed = seed * counter;
                let mut rng = SmallRng::seed_from_u64(computed_seed);

                let random_num: u64 = rng.gen();

                let msg = serialize_rust(Message::WakeUp(random_num));
                self.qs[id as usize].send(2, &msg.buffer)?;

                // old
                // let msg = serialize_rust(Message::WakeUp(self.cfg.random_number));
                // self.qs[id as usize].send(2, &msg.buffer)?;
            }
            Message::Finished(id) => {
                self.states[(id - 1) as usize] = State::Finished;
            }
            Message::Stuck(id) => {
                self.states[(id - 1) as usize] = State::Blocked;
            }
            Message::HasToSend4(id, if_id, ip, pkt_id) => {
                // self.cfg.topo.get_jitter(self.cfg.random_number,self.cfg.n_use_random_number);
                let n_use_random_number = self
                    .cfg
                    .n_use_random_number
                    .get_mut(usize::from(id) - 1)
                    .expect("Node ID not found");
                // println!("{:?}", self.cfg.topo);
                // println!("id {}, if_id {}, ip {:?}",id, if_id, &ip);
                let timestamp = current_time
                    + self.cfg.topo.get_delay_v4(id, if_id, &ip).unwrap()
                    + self
                        .cfg
                        .topo
                        .get_jitter(
                            self.cfg.random_number,
                            *n_use_random_number,
                            &self.cfg.jitter_distribution,
                        )
                        .unwrap()
                        * self.cfg.jitter_coef;
                *n_use_random_number += 1;
                if let Some(x) = self.events.get_mut(&timestamp) {
                    x.add_packet(id, pkt_id);
                } else {
                    self.events
                        .insert(timestamp, TimestampActions::new_pkt_id(id, pkt_id));
                }
            }
            Message::HasToSend6(id, if_id, ip, pkt_id) => {
                let n_use_random_number = self
                    .cfg
                    .n_use_random_number
                    .get_mut(usize::from(id))
                    .expect("Node ID not found");
                let timestamp = current_time
                    + self.cfg.topo.get_delay_v6(id, if_id, &ip).unwrap()
                    + self
                        .cfg
                        .topo
                        .get_jitter(
                            self.cfg.random_number,
                            *n_use_random_number,
                            &self.cfg.jitter_distribution,
                        )
                        .unwrap()
                        * self.cfg.jitter_coef;
                *n_use_random_number += 1;
                if let Some(x) = self.events.get_mut(&timestamp) {
                    x.add_packet(id, pkt_id);
                } else {
                    self.events
                        .insert(timestamp, TimestampActions::new_pkt_id(id, pkt_id));
                }
            }
            _ => {
                //TODO
            }
        }
        Ok(())
    }

    /// Part of a timespot where the delayed send are actually sent
    fn sending_time_loop(&self, ta: TimestampActions) -> Result<()> {
        for (process, pkt_id) in ta.flatten() {
            let msg = serialize_rust(Message::Send(pkt_id));

            self.qs[process as usize].send(1, &msg.buffer)?;

            let mut msg: Buffer = Buffer::new();
            if let Ok(_) = self.qs[0].recv(&mut msg.buffer) {
                if let Message::Sent(p, p_id) = deserialize_rust(msg) {
                    if p != process || p_id != pkt_id {
                        self.logs.log("error", &format!( "bad Sent received:\n\texpected: Sent({},{})\n\treceived: Sent({},{})", process, pkt_id, p, p_id));
                        panic!("error");
                    }
                } else {
                    self.logs.log(
                        "error",
                        &format!(
                            "bad message received:\n\texpected: Sent({},{})\n\treceived: {:?}",
                            process,
                            pkt_id,
                            deserialize_rust(msg)
                        ),
                    );
                    panic!("error");
                }
            } else {
                self.logs.log("error", "could not receive sent");
                panic!("error");
            }
        }
        Ok(())
    }

    /// Part of a timespot where the processes actually run
    fn running_time_loop(&mut self, current_time: u64) -> Result<State> {
        let mut msg: Buffer = Buffer::new();
        loop {
            match self.qs[0].recv(&mut msg.buffer) {
                Ok(_) => {
                    self.messages_handler(&msg, current_time)?;
                    let mut nb_blocked = 0;
                    let mut nb_finished = 0;
                    for s in self.states.iter_mut() {
                        match s {
                            State::Blocked => nb_blocked += 1,
                            State::Finished => nb_finished += 1,
                            _ => {}
                        }
                    }

                    if nb_finished == self.cfg.nb_follower {
                        return Ok(State::Finished);
                    }
                    if nb_blocked + nb_finished == self.cfg.nb_follower {
                        //all followers are blocked or finished, we need to make a step in time
                        return Ok(State::Blocked);
                    }
                }
                Err(e) => {
                    self.logs.log("error", "recv on leader queue failed : {e}");
                    eprintln!("Message error: {e}");
                    panic!("recv on leader queue failed");
                }
            }
        }
    }

    /// Main simulation loop. Advances time and processes events.
    fn main_loop(&mut self) -> Result<u8> {
        //inti logger

        let msg = serialize_rust(Message::WakeUp(0));
        for (s, q) in self.states.iter_mut().zip(self.qs[1..].iter()) {
            q.send(1, &msg.buffer)?;

            *s = State::Running;
        }

        if let Ok(State::Finished) = self.running_time_loop(0) {
            eprintln!("Simulation finished by all process finishing");
            return Ok(0);
        }

        loop {
            if let Some((time, ta)) = self.events.pop_first() {
                /*************** First half, sending time ***************/
                let _ = self.sending_time_loop(ta); //TODO : manage error (at least log it)

                /************** Second half, running time ***************/
                let msg = serialize_rust(Message::WakeUp(time));
                for (s, q) in self.states.iter_mut().zip(self.qs[1..].iter()) {
                    //TODO: if no send has been done, only wake up processes that need to be
                    // wake them up multiple times if needed
                    // update receive_msg() according to it (so after sending a signal it will go back in the receiving loop)
                    if s != &State::Finished {
                        q.send(1, &msg.buffer)?;
                        *s = State::Running;
                    }
                }
                if let Ok(State::Finished) = self.running_time_loop(time) {
                    eprintln!("Simulation finished by all process finishing");
                    return Ok(0);
                }
            } else {
                // the simulation is finished
                self.logs.log(
                    "info",
                    "Simulation finished by all process beeing blocked with no more events",
                );
                eprintln!("Simulation finished by all process beeing blocked with no more events");
                return Ok(1);
            }
        }
    }

    /// Starts the simulation: initializes queues, forks followers, and enters the main loop.
    fn run(mut self) {
        self.leader_init_queues()
            .expect("leader queues initialisation failed"); //open the communication queues

        // In Docker mode, create containers first and wire network before starting processes
        if self.cfg.mode == "docker" {
            let pids = Self::create_docker_containers(&self.cfg, &self.logs).expect("Failed to create containers");
            Self::wire_links_docker(&self.cfg, &pids, &self.logs).expect("Failed to wire docker links");
            // Store PIDs
            self.docker_pids = Some(pids);
        } else {
            for i in 0..self.cfg.nb_follower {
                //start the followers
                self.run_follower( // TODO: manage env
                                   (i + 1) as u8).expect("run follower failed");
                // TODO: add in the env the name of the queue
                //std::thread::sleep(std::time::Duration::from_millis(5000));
            }
        }

        self.main_loop().expect("main loop failed");
    }
}

impl Simulation {
    /// Create Docker containers running the actual application process.
    /// This merges container creation with process execution in Docker mode.
    /// The application will wait for JADE's signal before executing, so it's safe
    /// to start the process before network configuration is complete.
    fn create_docker_containers(cfg: &Config, logs: &Logger) -> Result<HashMap<u8, i32>> {
        let log_dir = env::var("LOG_DIR").unwrap_or_else(|_| "logs/".to_string());
        if !Path::new(&log_dir).exists() {
            std::fs::create_dir_all(&log_dir).expect("Failed to create log directory");
        }
        let log_dir_abs = std::fs::canonicalize(&log_dir).expect("Failed to canonicalize log directory");
        let log_dir_str = log_dir_abs.to_str().expect("Log directory path is not valid UTF-8");

        let mut pids: HashMap<u8, i32> = HashMap::new();
        for node in cfg.topo.grf.nodes.iter() {
            let id = node.id as u8;
            let exe_cfg = cfg.exe.iter().find(|p| p.node_ids.contains(&id))
                .or_else(|| cfg.exe.get((id - 1) as usize))
                .expect(&format!("Missing executable configuration for node ID {}", id));

            let img = exe_cfg.image.as_ref().expect(&format!("Missing image for node ID {}", id));
            let name = format!("jade_node_{}", id);

            // Stop any existing container
            let mut cmd_stop = Command::new("/usr/bin/env");
            cmd_stop.stdout(Stdio::null()).stderr(Stdio::null())
                .arg("docker").arg("stop").arg(&name);
            if let Ok(val) = env::var("DOCKER_HOST") {
                cmd_stop.env("DOCKER_HOST", val);
            }
            let _ = cmd_stop.status();

            // Start container with the actual application process
            // The process will wait for JADE's signal, so network can be configured after
            let mut cmd = Command::new("/usr/bin/env");
            cmd.arg("docker") //TODO: move interop specific args to config file
                .arg("run")
                .arg("-d")
                .arg("--privileged")
                .arg("--rm")
                .arg("--name").arg(&name)
                .arg("--ipc=host")
                .arg("--ulimit").arg("msgqueue=-1")
                .arg("--ulimit").arg("memlock=67108864")
                .arg("-e").arg("CRON=\"$CRON\"") //TODO: set value 
                .arg("-e").arg("ROLE=server") //TODO: set role
                .arg("-e").arg("SERVER_PARAMS=\"$SERVER_PARAMS\"") //TODO: set params
                .arg("-e").arg("SSLKEYLOGFILE=/logs/keys.log")
                .arg("-e").arg("QLOGDIR=/logs/qlog/")
                .arg("-e").arg("TESTCASE=\"$TESTCASE_SERVER\"") //TODO: set test case
                .arg("-v").arg("<WWW_DIR>:/www:ro") //TODO: set dir
                .arg("-v").arg("<CERTS_DIR>:/certs:ro") //TODO: set dir
                .arg("-v").arg(format!("{}:/logs", log_dir_str))
                .arg("-e").arg(format!("ID={}", id))
                .arg("-e").arg(format!("LD_PRELOAD={}", DOCKER_LIB_NAME))
                .arg("-e").arg("LOG_DIR=/logs/")
                .arg(img.as_c_str().to_str().unwrap());

            // Add the command and its arguments
            for arg in &exe_cfg.args {
                cmd.arg(arg.as_c_str().to_str().unwrap());
            }

            if let Ok(val) = env::var("DOCKER_HOST") {
                cmd.env("DOCKER_HOST", val);
            }

            // Log the full command
            let cmd_str = format!("{:?}", cmd);
            logs.log("info", &format!("Starting container {} with command: {}", name, cmd_str));
            eprintln!("Starting container {} with command: {}", name, cmd_str);

            let status = cmd.status();
            if status.is_err() || !status.unwrap().success() {
                logs.log("error", &format!("Failed to start container {}", name));
                return Err(Error::new(ErrorKind::Other, "docker run failed"));
            }

            // Get PID with retry
            let mut pid: i32 = 0;
            for attempt in 0..10 {
                if attempt > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                let mut cmd_inspect = Command::new("/usr/bin/env");
                cmd_inspect.arg("docker").arg("inspect").arg("-f").arg("{{.State.Pid}}").arg(&name);
                if let Ok(val) = env::var("DOCKER_HOST") {
                    cmd_inspect.env("DOCKER_HOST", val);
                }
                if let Ok(output) = cmd_inspect.output() {
                    if output.status.success() {
                        let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if let Ok(parsed_pid) = pid_str.parse::<i32>() {
                            if parsed_pid > 0 && std::path::Path::new(&format!("/proc/{}/ns/net", parsed_pid)).exists() {
                                pid = parsed_pid;
                                break;
                            }
                        }
                    }
                }
                if attempt == 9 {
                    // Container failed - get logs for debugging
                    let mut cmd_logs = Command::new("/usr/bin/env");
                    cmd_logs.arg("docker").arg("logs").arg(&name);
                    if let Ok(val) = env::var("DOCKER_HOST") {
                        cmd_logs.env("DOCKER_HOST", val);
                    }
                    if let Ok(log_output) = cmd_logs.output() {
                        let logs_str = String::from_utf8_lossy(&log_output.stdout);
                        let err_str = String::from_utf8_lossy(&log_output.stderr);
                        logs.log("error", &format!("Container {} logs stdout: {}", name, logs_str));
                        logs.log("error", &format!("Container {} logs stderr: {}", name, err_str));
                        eprintln!("Container {} failed. Stdout: {}", name, logs_str);
                        eprintln!("Container {} failed. Stderr: {}", name, err_str);
                    }

                    // Check if container is still running
                    let mut cmd_state = Command::new("/usr/bin/env");
                    cmd_state.arg("docker").arg("inspect").arg("-f").arg("{{.State.Status}}").arg(&name);
                    if let Ok(val) = env::var("DOCKER_HOST") {
                        cmd_state.env("DOCKER_HOST", val);
                    }
                    if let Ok(state_output) = cmd_state.output() {
                        let state = String::from_utf8_lossy(&state_output.stdout);
                        eprintln!("Container {} state: {}", name, state);
                    }

                    logs.log("error", &format!("Failed to get valid PID for container {}", name));
                    return Err(Error::new(ErrorKind::Other, "Failed to get valid container PID"));
                }
            }
            pids.insert(id, pid);
        }
        Ok(pids)
    }

    /// Wire containers with veth pairs and assign IPs (/32, /128) according to GML.
    fn wire_links_docker(cfg: &Config, pids: &HashMap<u8, i32>, logs: &Logger) -> Result<()> {
        // For each edge, create a veth pair and move each end into the corresponding container netns
        for edge in cfg.topo.grf.edges.iter() {
            let src: u8 = edge.source as u8;
            let dst: u8 = edge.target as u8;
            let if_src = format!("veth_{}_{}", src, dst);
            let if_dst = format!("veth_{}_{}", dst, src);

            // Create pair
            let pid_src = *pids.get(&src).expect("missing pid src");
            let pid_dst = *pids.get(&dst).expect("missing pid dst");
            let status = Command::new("/usr/bin/env").arg("ip").arg("link").arg("add")
                .arg(&if_src).arg("netns").arg(pid_src.to_string()).arg("type").arg("veth").arg("peer").arg(&if_dst)
                .arg("netns").arg(pid_dst.to_string()).status();
            if status.is_err() || !status.unwrap().success() {
                logs.log("error", &format!("Failed to create veth {} <-> {}", if_src, if_dst));
                return Err(Error::new(ErrorKind::Other, "ip link add failed"));
            }
            let _ = Command::new("/usr/bin/env").arg("ip").arg("link").arg("set").arg(&if_src).arg("netns").arg(pid_src.to_string()).status();
            let _ = Command::new("/usr/bin/env").arg("ip").arg("link").arg("set").arg(&if_dst).arg("netns").arg(pid_dst.to_string()).status();

            // Determine IPs from topology for each node
            let ipv4_source: Ipv4Addr = cfg
                .topo
                .ip4_node
                .iter()
                .find_map(|(key, &val)| if val.0 == src as u8 { Some(key.clone()) } else { None })
                .expect("Source IPv4 not found").try_into().unwrap();
            let ipv4_target: Ipv4Addr = cfg
                .topo
                .ip4_node
                .iter()
                .find_map(|(key, &val)| if val.0 == dst as u8 { Some(key.clone()) } else { None })
                .expect("Target IPv4 not found").try_into().unwrap();

            // Configure inside containers using nsenter
            let _ = Command::new("/usr/bin/env").arg("nsenter").arg("-t").arg(pid_src.to_string()).arg("-n")
                .arg("ip").arg("addr").arg("add").arg(format!("{}/24", ipv4_source)).arg("dev").arg(&if_src).status();
            let _ = Command::new("/usr/bin/env").arg("nsenter").arg("-t").arg(pid_src.to_string()).arg("-n")
                .arg("ip").arg("link").arg("set").arg(&if_src).arg("up").status();

            let _ = Command::new("/usr/bin/env").arg("nsenter").arg("-t").arg(pid_dst.to_string()).arg("-n")
                .arg("ip").arg("addr").arg("add").arg(format!("{}/24", ipv4_target)).arg("dev").arg(&if_dst).status();
            let _ = Command::new("/usr/bin/env").arg("nsenter").arg("-t").arg(pid_dst.to_string()).arg("-n")
                .arg("ip").arg("link").arg("set").arg(&if_dst).arg("up").status();

            if cfg.topo.ip6_node.len() != 0 {
                // Optional IPv6 configuration (/64)
                let ipv6_source: Ipv6Addr = cfg
                    .topo
                    .ip6_node
                    .iter()
                    .find_map(|(key, &val)| if val.0 == src as u8 { Some(key.clone()) } else { None })
                    .expect("Source IPv6 not found").try_into().unwrap();
                let ipv6_target: Ipv6Addr = cfg
                    .topo
                    .ip6_node
                    .iter()
                    .find_map(|(key, &val)| if val.0 == dst as u8 { Some(key.clone()) } else { None })
                    .expect("Target IPv6 not found").try_into().unwrap();
                let _ = Command::new("/usr/bin/env").arg("nsenter").arg("-t").arg(pid_src.to_string()).arg("-n")
                    .arg("ip").arg("-6").arg("addr").arg("add").arg(format!("{}/64", ipv6_source)).arg("dev").arg(&if_src).status();
                let _ = Command::new("/usr/bin/env").arg("nsenter").arg("-t").arg(pid_dst.to_string()).arg("-n")
                    .arg("ip").arg("-6").arg("addr").arg("add").arg(format!("{}/64", ipv6_target)).arg("dev").arg(&if_dst).status();
            }
        }
        Ok(())
    }
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    // Simple CLI parsing: --mode <mode>, --output-format=interop-runner, plus a config path
    let mut cli_mode: Option<String> = None;
    let mut _cli_output_format: Option<String> = None; // TODO: implement interop-runner output formatting

    // Remove binary name
    if !args.is_empty() { args.remove(0); }

    let mut i = 0usize;
    let mut cfg_path: Option<String> = None;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                if i + 1 < args.len() { cli_mode = Some(args[i + 1].clone()); i += 2; } else { eprintln!("--mode requires a value"); return; }
            }
            s if s.starts_with("--output-format=") => {
                let v = s.splitn(2, '=').nth(1).unwrap_or("").to_string();
                _cli_output_format = if v.is_empty() { None } else { Some(v) };
                i += 1;
            }
            s if s.starts_with("-") => {
                eprintln!("Unknown option: {}", s); return;
            }
            s => {
                cfg_path = Some(s.to_string());
                i += 1;
            }
        }
    }

    if cfg_path.is_none() {
        eprintln!("Please provide a config file");
        return;
    }

    let cfg = Config::new(Path::new(&cfg_path.unwrap())).expect("Error parsing config file");
    if let Some(ref cm) = cli_mode {
        if cm != &cfg.mode {
            eprintln!("Notice: CLI --mode={} ignored; TOML mode='{}' has precedence", cm, cfg.mode);
        }
    }

    let sim = Simulation::new(cfg);
    sim.run();
}

#[cfg(test)]
mod unit_testing {
    use ntest::timeout;
    use posixmq::PosixMq;
    use serial_test::{parallel, serial};
    use std::thread::{self};
    use std::time::Duration;

    use crate::*;
    use network_time_simulator::{Message::*, *};

    const NB_QUEUES_TEST: usize = 4;

    impl Config {
        fn default_config_nb(nb_follower: usize) -> Self {
            let mut cfg = Self::default_config();
            cfg.nb_follower = nb_follower;
            cfg.log_file = "logs/log.txt".to_string();
            return cfg;
        }
    }

    #[test]
    #[timeout(10000)]
    #[serial]
    fn test_init_queues() {
        let mut sim = Simulation::new(Config::default_config_nb(NB_QUEUES_TEST));
        sim.leader_init_queues().expect("creating leader queues");
        let mut buf = vec![0; 100];
        assert_eq!(sim.qs.len(), NB_QUEUES_TEST + 1);
        for i in 1..NB_QUEUES_TEST + 1 {
            let qio = follower_init_queues(i as u8, NB_QUEUES_TEST, &"/nts_test".to_string())
                .expect("creating follower queues");
            sim.qs[i]
                .send(0 as u32, b"Born?")
                .expect("send first message failed");
            assert_eq!(
                qio.0
                    .recv_timeout(&mut buf, Duration::from_secs(1))
                    .unwrap(),
                (0 as u32, "Born?".len())
            );
            qio.1
                .send(0 as u32, b"Yes!")
                .expect("send first message failed");
            assert_eq!(
                sim.qs[0]
                    .recv_timeout(&mut buf, Duration::from_secs(1))
                    .unwrap(),
                (0 as u32, "Yes!".len())
            );
        }
    }

    #[test]
    #[timeout(10000)]
    #[parallel]
    fn test_serialize_deserialize() {
        assert_eq!(deserialize_rust(serialize_rust(Stuck(42))), Stuck(42));
        assert_ne!(deserialize_rust(serialize_rust(Stuck(42))), Stuck(43));

        assert_eq!(
            deserialize_rust(serialize_rust(AddStep(42, 43))),
            AddStep(42, 43)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(AddStep(42, 43))),
            AddStep(42, 44)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(AddStep(42, 43))),
            AddStep(43, 43)
        );

        assert_eq!(
            deserialize_rust(serialize_rust(DelStep(42, 42))),
            DelStep(42, 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(DelStep(42, 42))),
            DelStep(42, 43)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(DelStep(42, 42))),
            DelStep(43, 42)
        );

        assert_eq!(
            deserialize_rust(serialize_rust(HasToSend4(
                42,
                0,
                Ipv4AddrC::new(42, 43, 44, 45),
                42
            ))),
            HasToSend4(42, 0, Ipv4AddrC::new(42, 43, 44, 45), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend4(
                42,
                0,
                Ipv4AddrC::new(42, 43, 44, 45),
                42
            ))),
            HasToSend4(43, 0, Ipv4AddrC::new(42, 43, 44, 45), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend4(
                42,
                0,
                Ipv4AddrC::new(42, 43, 44, 45),
                42
            ))),
            HasToSend4(42, 0, Ipv4AddrC::new(43, 43, 44, 45), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend4(
                42,
                0,
                Ipv4AddrC::new(42, 43, 44, 45),
                42
            ))),
            HasToSend4(42, 0, Ipv4AddrC::new(42, 44, 44, 45), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend4(
                42,
                0,
                Ipv4AddrC::new(42, 43, 44, 45),
                42
            ))),
            HasToSend4(42, 0, Ipv4AddrC::new(42, 43, 45, 45), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend4(
                42,
                0,
                Ipv4AddrC::new(42, 43, 44, 45),
                42
            ))),
            HasToSend4(42, 0, Ipv4AddrC::new(42, 43, 44, 46), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend4(
                42,
                0,
                Ipv4AddrC::new(42, 43, 44, 45),
                42
            ))),
            HasToSend4(42, 0, Ipv4AddrC::new(42, 43, 44, 45), 43)
        );

        assert_eq!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 43, 44, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(42, 0, Ipv6AddrC::new(42, 43, 43, 44, 46, 47, 48, 49), 42)
        );

        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(43, 0, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42)
        );

        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(42, 0, Ipv6AddrC::new(43, 43, 44, 45, 46, 47, 48, 49), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(42, 0, Ipv6AddrC::new(42, 44, 44, 45, 46, 47, 48, 49), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(42, 0, Ipv6AddrC::new(42, 43, 45, 45, 46, 47, 48, 49), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(42, 0, Ipv6AddrC::new(42, 43, 44, 46, 46, 47, 48, 49), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(42, 0, Ipv6AddrC::new(42, 43, 44, 45, 47, 47, 48, 49), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(42, 0, Ipv6AddrC::new(42, 43, 44, 45, 46, 48, 48, 49), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(42, 0, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 49, 49), 42)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(42, 0, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 50), 42)
        );

        assert_ne!(
            deserialize_rust(serialize_rust(HasToSend6(
                42,
                0,
                Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49),
                42
            ))),
            HasToSend6(42, 0, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 43)
        );

        assert_eq!(deserialize_rust(serialize_rust(Send(42))), Send(42));
        assert_ne!(deserialize_rust(serialize_rust(Send(42))), Send(43));

        assert_eq!(deserialize_rust(serialize_rust(Sent(1, 42))), Sent(1, 42));
        assert_ne!(deserialize_rust(serialize_rust(Sent(1, 42))), Sent(1, 43));
        assert_ne!(deserialize_rust(serialize_rust(Sent(1, 42))), Sent(2, 42));

        assert_eq!(deserialize_rust(serialize_rust(GetTime(42))), GetTime(42));
        assert_ne!(deserialize_rust(serialize_rust(GetTime(42))), GetTime(43));

        assert_eq!(
            deserialize_rust(serialize_rust(GetRand(42, 0))),
            GetRand(42, 0)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(GetRand(42, 0))),
            GetRand(43, 0)
        );
        assert_ne!(
            deserialize_rust(serialize_rust(GetRand(42, 0))),
            GetRand(42, 1)
        );

        assert_eq!(deserialize_rust(serialize_rust(WakeUp(42))), WakeUp(42));
        assert_ne!(deserialize_rust(serialize_rust(WakeUp(42))), WakeUp(43));

        assert_eq!(deserialize_rust(serialize_rust(Finished(42))), Finished(42));
        assert_ne!(deserialize_rust(serialize_rust(Finished(42))), Finished(43));
    }

    #[test]
    #[timeout(10000)]
    #[serial]
    fn test_serialize_deserialize_mq() {
        let mut sim = Simulation::new(Config::default_config_nb(1));
        sim.leader_init_queues().expect("leader queues creating");
        let qio =
            follower_init_queues(1, 1, &"/nts_test".to_string()).expect("follower queues creating");
        let mut msg: Buffer = Buffer::new();
        let msgs = [
            Stuck(42),
            AddStep(42, 43),
            HasToSend4(42, 0, Ipv4AddrC::new(42, 43, 44, 45), 42),
            HasToSend6(42, 0, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42),
            Send(42),
            Sent(1, 42),
            DelStep(42, 42),
            GetTime(42),
            GetRand(42, 0),
            Finished(42),
        ];

        for m in msgs {
            let check = m.clone();
            qio.1
                .send(SIZE_BUFFER as u32, &serialize_rust(m).buffer)
                .expect("send follower message failed");
            sim.qs[0].recv(&mut msg.buffer).unwrap();
            assert_eq!(deserialize_rust(msg), check);
        }
        sim.qs[1]
            .send(1, &serialize_rust(WakeUp(42)).buffer)
            .expect("send leader message failed");
        qio.0.recv(&mut msg.buffer).unwrap();
        assert_eq!(deserialize_rust(msg), WakeUp(42));
    }

    fn mini_setup(nb_add_step: u64, nb_stuck: u64, nb_recv: u64, return_value: u8) {
        let mut sim = Simulation::new(Config::default_config());
        sim.leader_init_queues().expect("leader queues creating");
        let qf1 =
            follower_init_queues(1, 2, &"/nts_test".to_string()).expect("creating follower queues");
        let qf2 =
            follower_init_queues(2, 2, &"/nts_test".to_string()).expect("creating follower queues");
        let t = thread::spawn(move || sim.main_loop());
        let mut msg: Buffer = Buffer::new();
        qf1.0.recv(&mut msg.buffer).unwrap();
        assert_eq!(deserialize_rust(msg), WakeUp(0)); // checks that the first WakeUp message is sent
        qf2.0.recv(&mut msg.buffer).unwrap();
        assert_eq!(deserialize_rust(msg), WakeUp(0));
        for i in 1..nb_add_step + 1 {
            qf1.1
                .send(1, &serialize_rust(AddStep(1, i)).buffer)
                .expect("send add_step failed");
        }
        for _i in 1..nb_stuck + 1 {
            qf1.1
                .send(1, &serialize_rust(Stuck(1)).buffer)
                .expect("send stuck failed");
            qf2.1
                .send(1, &serialize_rust(Stuck(2)).buffer)
                .expect("send stuck failed");
        }

        for i in 1..nb_recv + 1 {
            let mut msg: Buffer = Buffer::new();
            qf1.0.recv(&mut msg.buffer).unwrap();
            assert_eq!(deserialize_rust(msg), WakeUp(i));
            qf2.0.recv(&mut msg.buffer).unwrap();
            assert_eq!(deserialize_rust(msg), WakeUp(i));
        }
        qf1.1
            .send(1, &serialize_rust(Finished(1)).buffer)
            .expect("send finished failed");
        qf2.1
            .send(1, &serialize_rust(Finished(2)).buffer)
            .expect("send finished failed");
        if let Ok(ret) = t.join().unwrap() {
            assert_eq!(ret, return_value);
        } else {
            assert!(false);
        }
    }

    #[test]
    #[timeout(10000)]
    #[serial]
    fn direct_finished() {
        mini_setup(0, 0, 0, 0);
    }

    #[test]
    #[timeout(30000)]
    #[serial]
    fn direct_blocked() {
        mini_setup(0, 1, 0, 1);
    }

    #[test]
    #[timeout(10000)]
    #[serial]
    fn one_step() {
        mini_setup(1, 1, 1, 0);
    }

    #[test]
    #[timeout(10000)]
    #[serial]
    fn too_much_steps() {
        mini_setup(2, 1, 1, 0);
    }

    #[test]
    #[timeout(10000)]
    #[parallel]
    fn test_messages_handler() {
        // Setting up the environment
        let mut followers_qs: Vec<PosixMq> = vec![];
        const SIZE_TEST: u8 = 4;
        let mut sim = Simulation::new(Config::default_config());
        sim.cfg.nb_follower = SIZE_TEST as usize;
        for i in 0..sim.cfg.nb_follower {
            //start the followers
            if let Ok((qi, _)) =
                follower_init_queues(i as u8, sim.cfg.nb_follower, &"/nts_test".to_string())
            {
                followers_qs.push(qi);
            }
        }
        sim.leader_init_queues().unwrap();

        //test with one timestamp
        sim.messages_handler(&serialize_rust(Message::AddStep(1, 1)), 0)
            .expect("message handler failing");
        if let Some((&1, v2)) = sim.events.first_key_value() {
            assert!(v2.contain_process(1));
            assert_eq!(v2.nb_process(), 1);
        } else {
            assert!(false);
        }

        let _ = sim.messages_handler(&serialize_rust(Message::AddStep(2, 1)), 0);
        if let Some((&1, v2)) = sim.events.first_key_value() {
            assert!(v2.contain_process(1));
            assert!(v2.contain_process(2));
            assert_eq!(v2.nb_process(), 2);
        } else {
            assert!(false);
        }

        let _ = sim.messages_handler(&serialize_rust(Message::AddStep(2, 1)), 0);
        if let Some((&1, v2)) = sim.events.first_key_value() {
            let mut ta = v2.clone();
            assert!(v2.contain_process(1));
            assert!(v2.contain_process(2));
            assert_eq!(v2.nb_process(), 3);
            ta.del_process(2);
            assert!(ta.contain_process(2));
        } else {
            assert!(false);
        }

        let _ = sim.messages_handler(&serialize_rust(Message::DelStep(1, 1)), 0);
        if let Some((&1, v2)) = sim.events.first_key_value() {
            assert!(!v2.contain_process(1));
            assert!(v2.contain_process(2));
            assert_eq!(v2.nb_process(), 2);
        } else {
            assert!(false);
        }
    }

    #[test]
    #[serial]
    fn test_random() {
        let mut sim = Simulation::new(Config::default_config());

        sim.leader_init_queues().expect("leader queues creating");
        let qf1 =
            follower_init_queues(1, 2, &"/nts_test".to_string()).expect("creating follower queues");

        let qf2 =
            follower_init_queues(2, 2, &"/nts_test".to_string()).expect("creating follower queues");

        let _t = thread::spawn(move || sim.main_loop());
        let mut msg: Buffer = Buffer::new();

        qf1.0.recv(&mut msg.buffer).unwrap();
        qf2.0.recv(&mut msg.buffer).unwrap();

        qf1.1
            .send(1, &serialize_rust(GetRand(1, 34)).buffer)
            .expect("send get_rand failed");
        let mut a = Buffer::new();
        qf1.0.recv(&mut a.buffer).unwrap();
        qf1.1
            .send(1, &serialize_rust(GetRand(1, 34)).buffer)
            .expect("send get_rand failed");
        let mut b = Buffer::new();
        qf1.0.recv(&mut b.buffer).unwrap();
        assert_ne!(deserialize_rust(a), deserialize_rust(b));
        qf2.1
            .send(1, &serialize_rust(GetRand(2, 34)).buffer)
            .expect("send get_rand failed");
        let mut c = Buffer::new();
        qf2.0.recv(&mut c.buffer).unwrap();
        assert_eq!(deserialize_rust(a), deserialize_rust(c));
        qf1.1
            .send(1, &serialize_rust(GetRand(1, 52)).buffer)
            .expect("send get_rand failed");
        let mut d = Buffer::new();
        qf1.0.recv(&mut d.buffer).unwrap();
        assert_ne!(deserialize_rust(a), deserialize_rust(d));
        qf1.1
            .send(1, &serialize_rust(GetRand(1, 34)).buffer)
            .expect("send get_rand failed");
        let mut e = Buffer::new();
        qf1.0.recv(&mut e.buffer).unwrap();
        qf1.1
            .send(1, &serialize_rust(GetRand(1, 34)).buffer)
            .expect("send get_rand failed");
        let mut f = Buffer::new();
        qf1.0.recv(&mut f.buffer).unwrap();
        assert_eq!(deserialize_rust(a), deserialize_rust(e));
        assert_eq!(deserialize_rust(b), deserialize_rust(f));
    }
}

#[cfg(test)]
mod determinism {
    use helper::{cstringify, NetworkTopology, Process};
    use ntest::timeout;
    use posixmq::PosixMq;
    use serial_test::serial;
    use std::io::Result;
    use std::thread::{self, ScopedJoinHandle};
    use std::time::Duration;

    use crate::*;
    use network_time_simulator::{Message::*, *};

    macro_rules! NB_FOLLOWERS {
        () => {
            10 //must be root to get higher than 10
        };
    }

    const NONE_TPM: Option<(PosixMq, PosixMq)> = None;
    fn custom_arr_tpm(
        clo: impl Fn(u8) -> (PosixMq, PosixMq),
    ) -> [Option<(PosixMq, PosixMq)>; NB_FOLLOWERS!()] {
        let mut tab: [Option<(PosixMq, PosixMq)>; NB_FOLLOWERS!()] = [NONE_TPM; NB_FOLLOWERS!()];
        for i in 0..NB_FOLLOWERS!() {
            tab[i] = Some(clo(i as u8));
        }
        tab
    }

    const NONE_THREAD: Option<ScopedJoinHandle<Result<(u32, usize)>>> = None;

    /**
     * Run one thread by follower, checks if they've got a message,
     * asserts that non-prioritized follower haven't received any message
     **/
    macro_rules! m_receive {
        ($s:ident, $qfs:ident, $msgs:ident, $prioritized:ident, $tab:ident, $pkt_id:ident) => {
            {
                for i in 0..NB_FOLLOWERS!() {
                    let q: &Option<(PosixMq, PosixMq)> = &$qfs[i as usize];
                    $tab[i] = Some($s.spawn(move || {
                            match q.as_ref().unwrap().0.recv_timeout(&mut $msgs[i as usize].buffer, Duration::from_secs(1)) {
                                Ok(x) => {
                                    match $msgs[i as usize].into() {
                                        Send(id) => {
                                            assert_eq!(id, $pkt_id);
                                            if i as usize > $prioritized {
                                                eprintln!("Non-prioritized thread receive order to send: Send({})", id);
                                                assert!(false);
                                            }
                                            Ok(x)
                                        },
                                        m => {
                                            eprintln!("Bad message received: {:?}", m);
                                            assert!(false); // should not receive anything else
                                            Ok((0,0))
                                        },
                                    }

                                },
                                Err(_) => {
                                    if i as usize > $prioritized {
                                        assert!(true);
                                    }
                                    Ok((0,0))
                                },
                            }
                    }));
                }
            }
        };
    }

    /**
     * Joins the receiving threads
     * Asserts that one and only one follower received a message
     * Set id_order if unset, asserts it's unchanged otherwise
     * Sends a Sent to the leader to acknoledge the packet has been sent
     */
    macro_rules! m_check {
        ($ts:ident, $id_order:expr, Sent(_, $pkt_id:expr), $qfs:ident) => {
            let mut already_received = false;
            let mut id_received = usize::MAX;
            for i in 0..NB_FOLLOWERS!() {
                match $ts[i].take().expect("Uninit thread handle").join().unwrap() {
                    Ok((0,0)) => {
                        assert!(true); //all is ok
                    },
                    Ok(_) => {
                        assert!(!already_received);
                        already_received = true;
                        if $id_order == 0 {
                            $id_order = i;
                        } else {
                            assert_eq!($id_order, i);
                            $id_order = 0;
                        }
                        id_received = i;
                    },
                    Err(_) => {
                        assert!(false);
                    },
                };
            }
            assert!(id_received != usize::MAX); // checks that a follower received a message
            m_send!(Sent(id_received+1, $pkt_id), $qfs); // sends a Sent to the leader to acknowledge the packet has been sent
        };
    }

    macro_rules! m_send_1arg {
        ($msg:expr, $qfs:ident, $nb:expr) => {
            for i in 0..NB_FOLLOWERS!() {
                $qfs[i]
                    .as_ref()
                    .unwrap()
                    .1
                    .send(1, &serialize_rust($msg((i + 1) as u8)).buffer)
                    .expect("send message failed");
            }
        };
    }
    #[allow(unused_macros)]
    macro_rules! m_send_2arg {
        ($msg:expr, $arg:expr, $qfs:ident, $nb:expr) => {
            for i in 0..$nb {
                $qfs[i]
                    .as_ref()
                    .unwrap()
                    .1
                    .send(1, &serialize_rust($msg((i + 1) as u8, $arg)).buffer)
                    .expect("send message failed");
            }
        };
    }
    macro_rules! m_send_4arg {
        ($msg:expr, $dest:expr, $pkt_id:expr, $qfs:ident, $nb:expr) => {
            for i in 0..$nb {
                $qfs[i]
                    .as_ref()
                    .unwrap()
                    .1
                    .send(
                        1,
                        &serialize_rust($msg((i + 1) as u8, 0, $dest, $pkt_id)).buffer,
                    )
                    .expect("send message failed");
            }
        };
    }

    /**
     * Sends a message from all followers
     */
    macro_rules! m_send {
        (Stuck(_), $qfs:ident, $nb:expr) => {
            m_send_1arg!(Stuck, $qfs, $nb)
        };
        (Finished(_), $qfs:ident, $nb:expr) => {
            m_send_1arg!(Finished, $qfs, $nb)
        };
        (DelStep($id:expr, $step:expr), $qfs:ident) => {
            $qfs[$id - 1]
                .as_ref()
                .unwrap()
                .1
                .send(1, &serialize_rust(DelStep(($id) as u8, $step)).buffer)
                .expect("send DelStep failed");
        };
        (DelStep(_, $step:expr), $qfs:ident, $nb:expr) => {
            m_send_2arg!(DelStep, $step, $qfs, $nb)
        };
        (AddStep(_, $step:expr), $qfs:ident, $nb:expr) => {
            m_send_2arg!(AddStep, $step, $qfs, $nb)
        };
        (HasToSend4(_, $dest:expr, $pkt_id:expr), $qfs:ident, $nb:expr) => {
            m_send_4arg!(HasToSend4, $dest, $pkt_id, $qfs, $nb)
        };
        (HasToSend6(_, $dest:expr, $pkt_id:expr), $qfs:ident, $nb:expr) => {
            m_send_4arg!(HasToSend6, $dest, $pkt_id, $qfs, $nb)
        };
        (Sent($id:expr, $pkt_id:expr), $qfs:ident) => {
            $qfs[$id - 1]
                .as_ref()
                .unwrap()
                .1
                .send(1, &serialize_rust(Sent(($id) as u8, $pkt_id)).buffer)
                .expect("send DelStep failed");
        };
    }

    impl NetworkTopology {
        /**
         * Return a star centred on 10.0.0.10
         */
        fn test_topo(nb: usize) -> Self {
            let mut nodes: String = String::from(&format!(
                "  node [
    id {}
    label \"Node {}\"
    interface [
      id 0
      label \"wlp4s0\"
      ip [
        type \"v4\"
        ip \"10.0.0.{}\"
      ]
    ]
  ]
",nb+1,nb+1,nb+1),
            );
            let mut edges: String = String::new();
            for i in 1..nb + 1 {
                nodes.push_str(&format!(
                    "  node [
    id {}
    label \"Node {}\"
    interface [
      id 0
      label \"wlp4s0\"
      ip [
        type \"v4\"
        ip \"10.0.0.{}\"
      ]
    ]
  ]
",
                    i, i, i
                ));
                edges.push_str(&format!(
                    "  edge [
    source {}
    source_if 0
    target {}
    target_if 0
    label \"Link\"
    metric 20
    type \"symmetric\"
  ]
",
                    i,nb+1
                ));
            }
            let final_graph = format!(
                "graph [
  label \"test\"
  id 4
{}{}]",
                nodes, edges
            );
            return Self::load(&final_graph);
        }
    }

    impl Config {
        fn test_config(nb: usize, jitter: String) -> Self {
            const QNAME: &str = "/nts_test";
            const EXE_NAMES: [&CStr; 2] = [c"./client", c"./server"];
            const EXE1_ARGS: [&CStr; 5] = [c"./client", c"-i", c"127.0.0.1", c"-p", c"4443"];
            const EXE2_ARGS: [&CStr; 5] = [c"./server", c"-i", c"127.0.0.1", c"-p", c"4443"];
            const RANDOM_NUMBER: u64 = 84;
            let processes = vec![
                Process::new(
                    CString::from(EXE_NAMES[0]),
                    CString::from(EXE_NAMES[0]),
                    cstringify(&EXE1_ARGS),
                    vec![],
                ),
                Process::new(
                    CString::from(EXE_NAMES[1]),
                    CString::from(EXE_NAMES[1]),
                    cstringify(&EXE2_ARGS),
                    vec![],
                ),
            ];
            let mut n_use_random_number = Vec::with_capacity(nb);
            for _i in 0..nb {
                n_use_random_number.push(0);
            }

            return Self {
                nb_follower: nb,
                _qname: QNAME.to_string(),
                mode: "netns".to_string(),
                exe: processes,
                random_number: RANDOM_NUMBER,
                n_use_random_number: n_use_random_number,
                jitter_distribution: jitter,
                jitter_coef: 10,
                topo: NetworkTopology::test_topo(nb),
                log_level: vec!["trace".to_string(), "debug".to_string(), "warn".to_string(), "error".to_string(), "info".to_string(), "message".to_string()],
                log_file: "logs/log.txt".to_string(),
            };
        }
    }

    /**
     * In a topology of NB_FOLLOWERS processes, nb_sender of them send messages that should be received at the same time.
     * This test checks that senders are woken up one at a time, following a deterministic order.
     */
    fn test_determinism_setup(nb_sender: usize, jitter: Option<&str>) { // TODO add , jitter: Option<&str> to test determinism of both jitters
        //let mut already_received = false;

        for _iter in 0..2 {
            //Setting up the environment
            let mut sim = Simulation::new(Config::test_config(NB_FOLLOWERS!(), jitter.unwrap_or("no_jitter").to_string()));
            let mut msgs: [Buffer; NB_FOLLOWERS!() as usize] = [Buffer::new(); NB_FOLLOWERS!()];
            let nb_f = NB_FOLLOWERS!();
            let clo = |mut i| -> (PosixMq, PosixMq) {
                i += 1;
                follower_init_queues(i as u8, NB_FOLLOWERS!(), &"/nts_test".to_string())
                    .expect("follower queues creating")
            };
            let qfs: [Option<(PosixMq, PosixMq)>; NB_FOLLOWERS!()] = custom_arr_tpm(clo);
            let pkt_id = 1;
            thread::scope(|sc| {
                sim.leader_init_queues().expect("leader queues creating");
                let t = sc.spawn(|| sim.main_loop());
                let mut receiving_order: [usize; NB_FOLLOWERS!()] = [0; NB_FOLLOWERS!()];

                m_send!(
                    HasToSend4(_, Ipv4AddrC::new(10, 0, 0, NB_FOLLOWERS!()+1), pkt_id),
                    qfs,
                    nb_sender
                );
                m_send!(Stuck(_), qfs, nb_f);

                for i in 0..NB_FOLLOWERS!() {
                    let Ok(_) = qfs[i as usize]
                        .as_ref()
                        .unwrap()
                        .0
                        .recv(&mut msgs[i as usize].buffer)
                    else {
                        panic!("First received message should be WakeUp(0). Bad recv");
                    };
                    match msgs[i as usize].into() {
                        WakeUp(0) => {} //Ok
                        WakeUp(t) => {
                            panic!(
                                "First received message should be WakeUp(0). Bad time: {:?}",
                                t
                            );
                        }
                        m => {
                            panic!(
                                "First received message should be WakeUp(0). Bad msg: {:?}",
                                m
                            );
                        }
                    }
                }

                for i in 0..nb_sender {
                    thread::scope(|s| {
                        let mut ts: [Option<ScopedJoinHandle<Result<(u32, usize)>>>;
                            NB_FOLLOWERS!()] = [NONE_THREAD; NB_FOLLOWERS!()];
                        m_receive!(s, qfs, msgs, nb_sender, ts, pkt_id);
                        m_check!(ts, receiving_order[i], Sent(_, pkt_id), qfs);
                    });
                }

                for i in 0..NB_FOLLOWERS!() {
                    let Ok(_) = qfs[i as usize]
                        .as_ref()
                        .unwrap()
                        .0
                        .recv(&mut msgs[i as usize].buffer)
                    else {
                        panic!("First received message should be WakeUp(0). Bad recv");
                    };

                    if let Some(_) = jitter {
                        match msgs[i as usize].into() {
                            WakeUp(20) => {} //Ok
                            WakeUp(t) => {
                                panic!(
                                    "Third received message should be WakeUp(20). Bad time: {:?}",
                                    t
                                );
                            }
                            m => {
                                panic!(
                                    "Third received message should be WakeUp(20). Bad msg: {:?}",
                                    m
                                );
                            }
                        }
                    }
                }

                thread::scope(|s| {
                    let mut ts: [Option<ScopedJoinHandle<Result<(u32, usize)>>>; NB_FOLLOWERS!()] =
                        [NONE_THREAD; NB_FOLLOWERS!()];
                    m_receive!(s, qfs, msgs, nb_f, ts, pkt_id);
                    for i in 0..nb_f {
                        let _ = ts[i].take().expect("uninit thread handle").join().unwrap();
                    }
                });
                m_send!(Finished(_), qfs, nb_f);
                let _ = t.join();
            });
        }
    }

    #[test]
    #[serial]
    #[timeout(100000)]
    fn test_determinism() {
        test_determinism_setup(9, None);
    }
}
