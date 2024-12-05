pub mod helper;

use fork::{fork, Fork};
use futures::executor::block_on;
use futures::TryStreamExt;
use helper::{Config, TimestampActions};
use netns_rs::NetNs;
use network_time_simulator::SIZE_BUFFER;
use network_time_simulator::{deserialize_rust, serialize_rust, Buffer, Message};
use nix::unistd::execve;
use posixmq::PosixMq;
use rtnetlink::new_connection;
use std::collections::BTreeMap;
use std::env;
use std::ffi::{CStr, CString};
use std::io::Result;
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::fd::AsRawFd;
use std::path::Path;

#[derive(Copy, Clone, PartialEq, Debug)]
enum State {
    Running,
    Blocked,
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

/**
 * Open one queue on which the follower will send message to the leader and
 * one queue on which the follower will receive messages from the leader
 * @arg id: the id of the follower, strictly positive
 * @arg nb: the size of the queue (use the number of followers)
 * @return: on success return a pair of posix queues (follower_receiving_queue, follower_sending_queue)
 */
fn follower_init_queues(id: u8, nb: usize, qname: &String) -> Result<(PosixMq, PosixMq)> {
    // TODO: no more use
    // TODO: (for later) find a way to use this to avoid passing through the env
    //println!("id: {}, nb: {}, qname: {:?}", id, nb, qname);
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
    //println!("follower_init_queues: qi {:?}, qo {:?}", qi, qo);
    return Ok((qi, qo));
}

#[derive(Debug)]
pub struct Simulation {
    cfg: Config,
    states: Vec<State>,
    events: BTreeMap<u64, TimestampActions>,
    qs: Vec<PosixMq>,
}

impl Drop for Simulation {
    fn drop(&mut self) {
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
        let _ = self.cfg.unlink_queues();
    }
}

impl Simulation {
    fn new(cfg: Config) -> Self {
        let nb_f = cfg.nb_follower;
        let _ = cfg.unlink_queues();
        let btm = BTreeMap::new();

        let cfg = tokio::runtime::Runtime::new()
            .expect("Failed to create runtime")
            .block_on(Self::create_namespaces(cfg))
            .expect("Failed to create namespaces");

        Self {
            cfg,
            states: vec![State::Running; nb_f],
            events: btm,
            qs: Vec::with_capacity(nb_f + 1),
        }
    }

    /**
     * Create one namespace for each node and one link for each edge. Attach the interface to
     * the corresponding namespace and sets the interfaces up
     * @arg cfg: the configuration of the current simulation
     * @return: the configuration of the current simulation
     **/
    async fn create_namespaces(cfg: Config) -> Result<Config> {
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

        let (connection, handle, _) = new_connection().unwrap();
        tokio::spawn(connection);

        // Add the links between the namespaces
        for edge in cfg.topo.grf.edges.iter() {
            // Interface names
            // Interface veth_sourceNode_targetNode is the interface of sourceNode that is connected to targetNode
            let name1 = format!("veth_{}_{}", edge.source, edge.target);
            let name2 = format!("veth_{}_{}", edge.target, edge.source);

            // Create veth pair
            let request = handle.link().add().veth(name1.clone(), name2.clone());
            if let Err(error) = request.execute().await.map_err(|e| format!("{}", e)) {
                println!("Could not create veth pair: {}", error);
            }

            // Get interface index
            let links1: Vec<_> = handle
                .link()
                .get()
                .match_name(name1.clone())
                .execute()
                .try_collect()
                .await
                .map_err(|e| {
                    Error::new(
                        ErrorKind::Other,
                        format!("Failed to get interface information: {}", e),
                    )
                })?;

            let link1 = links1.into_iter().next().ok_or_else(|| {
                Error::new(
                    ErrorKind::NotFound,
                    format!("Interface {} not found", name1),
                )
            })?;

            let links2: Vec<_> = handle
                .link()
                .get()
                .match_name(name2.clone())
                .execute()
                .try_collect()
                .await
                .map_err(|e| {
                    Error::new(
                        ErrorKind::Other,
                        format!("Failed to get interface information: {}", e),
                    )
                })?;

            let link2 = links2.into_iter().next().ok_or_else(|| {
                Error::new(
                    ErrorKind::NotFound,
                    format!("Interface {} not found", name1),
                )
            })?;

            // Get namespace
            let source_ns = NetNs::get(edge.source.to_string()).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to get source namespace: {}", e),
                )
            })?;

            let target_ns = NetNs::get(edge.target.to_string()).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to get target namespace: {}", e),
                )
            })?;

            // Get the name space file descriptor
            let source_ns_fd = source_ns.file().as_raw_fd();
            let target_ns_fd = target_ns.file().as_raw_fd();

            // Get ip addresses
            // ipv4
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

            //add the interface index to the topology
            // cfg.topo.ip4_node.entry(ipv4addrc_source.clone()).and_modify(|e| {
            //     *e = (edge.source.try_into().unwrap(), link1.header.index.try_into().unwrap());
            // });

            // cfg.topo.ip4_node.entry(ipv4addrc_target.clone()).and_modify(|e| {
            //     *e = (edge.target.try_into().unwrap(), link2.header.index.try_into().unwrap());
            // });

            // println!("{:?}\n\n",cfg.topo.ip4_node);

            // ipv6
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

            // Put the interface in the correspondig namespace and set it up
            let mut link_set_req1 = handle.link().set(link1.header.index);
            link_set_req1 = link_set_req1.setns_by_fd(source_ns_fd);
            link_set_req1 = link_set_req1.up();
            link_set_req1.execute().await.map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!(
                        "Failed to move {} to namespace {}: {}",
                        name1, edge.source, e
                    ),
                )
            })?;

            let mut link_set_req2 = handle.link().set(link2.header.index);
            link_set_req2 = link_set_req2.setns_by_fd(target_ns_fd);
            link_set_req2 = link_set_req2.up();
            link_set_req2.execute().await.map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!(
                        "Failed to move {} to namespace {}: {}",
                        name2, edge.target, e
                    ),
                )
            })?;
            

            //go to the right namespace
            //this makes the simulation finish by all process being blocked with no more events
            //todo try with run instead of enter
            if let Err(e) = source_ns.enter() {
                println!("Error {:?} when entering the namespace {}", e, edge.source);
            }

            //create new handle
            let (connection_source, handle_source, _) = new_connection().unwrap();
            tokio::spawn(connection_source);

            // add the ip address to the interface
            //ipv4
            let addr_req1 =
                handle_source
                    .address()
                    .add(link1.header.index, IpAddr::V4(ipv4_source), 24);
            if let Err(e) = block_on(addr_req1.execute()) {
                println!(
                    "Error when add the ipv4 to the interface {:?} : {:?}",
                    name1, e
                );
            }

            //ipv6
            let addr_req1 =
                handle_source
                    .address()
                    .add(link1.header.index, IpAddr::V6(ipv6_source), 64);
            if let Err(e) = block_on(addr_req1.execute()) {
                println!(
                    "Error when add the ipv6 to the interface {:?} : {:?}",
                    name1, e
                );
            }

            //go to the right namespace
            if let Err(e) = target_ns.enter() {
                println!(
                    "Error {:?} when entering the namespace {:?}",
                    e, edge.target
                );
            }

            //create new handle
            let (connection_target, handle_target, _) = new_connection().unwrap();
            tokio::spawn(connection_target);

            // add the ip address to the interface
            //ipv4
            let addr_req2 =
                handle_target
                    .address()
                    .add(link2.header.index, IpAddr::V4(ipv4_target), 24);
            if let Err(e) = block_on(addr_req2.execute()) {
                println!(
                    "Error when add the ipv4 to the interface {:?} : {:?}",
                    name2, e
                );
            }

            //ipv6
            let addr_req2 =
                handle_target
                    .address()
                    .add(link2.header.index, IpAddr::V6(ipv6_target), 64);
            if let Err(e) = block_on(addr_req2.execute()) {
                println!(
                    "Error when add the ipv6 to the interface {:?} : {:?}",
                    name2, e
                );
            }
        }
        Ok(cfg)
    }

    /**
     * Starts a follower with the queues to communicate toward the leader as file descriptor 3 and from
     * the leader as file descriptor 4.
     * @arg id: the id of the follower
     * @arg exe: the path to the follower executable
     * @arg args: the arguments to start the follower
     * @arg env: the environment to start the follower
     * @return: the pid of the child on success
     **/
    fn run_follower(&self, id: u8, env: &[&CStr]) -> Result<i32> {
        match fork() {
            Ok(Fork::Parent(child)) => Ok(child),
            Ok(Fork::Child) => {
                let _ = follower_init_queues(id, self.cfg.nb_follower, &self.cfg._qname);
                execve(
                    &self.cfg.exe[(id - 1) as usize].path,
                    &self.cfg.exe[(id - 1) as usize].args,
                    env,
                )?;
                Ok(0)
            }
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from("Fork failed"),
            )),
        }
    }

    /**
     * Initialize one queue on which the leader will wait for the messages from the followers and a
     * queue by follower
     * @arg nb: the number of follower
     * @return: returns a Result
     **/
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

    fn messages_handler(&mut self, message: &Buffer, current_time: u64) -> Result<()> {
        match deserialize_rust(*message) {
            Message::AddStep(id, t) => {
                //println!("Adding a step {} for node {}", t, id);
                if let Some(x) = self.events.get_mut(&t) {
                    x.add_process(id);
                } else {
                    self.events.insert(t, TimestampActions::new_process(id));
                }
            }
            Message::DelStep(id, t) => {
                //println!("Deleting a step {} for node {}", t, id);
                if let Some(x) = self.events.get_mut(&t) {
                    x.del_process(id);
                }
            }
            Message::GetTime(id) => {
                //return head key of the BTreeMap
                //println!("Getting time for node {}", id);
                let msg = serialize_rust(Message::WakeUp(current_time));
                self.qs[id as usize].send(2, &msg.buffer)?;
            }
            Message::GetRand(id) => {
                //println!("Getting random for node {}", id);
                let msg = serialize_rust(Message::WakeUp(self.cfg.random_number));
                self.qs[id as usize].send(2, &msg.buffer)?;
            }
            Message::Finished(id) => {
                //println!("Node {} has finished", id);
                self.states[(id - 1) as usize] = State::Finished;
            }
            Message::Stuck(id) => {
                //println!("Node {} is stuck", id);
                self.states[(id - 1) as usize] = State::Blocked;
            }
            Message::HasToSend4(id, if_id, ip, pkt_id) => {
                //println!("Node {} has to send packet {} via {} to {:?}", id, pkt_id, if_id, ip);
                let timestamp = current_time + self.cfg.topo.get_delay_v4(id, if_id, &ip).unwrap();
                if let Some(x) = self.events.get_mut(&timestamp) {
                    x.add_packet(id, pkt_id);
                } else {
                    self.events
                        .insert(timestamp, TimestampActions::new_pkt_id(id, pkt_id));
                }
            }
            Message::HasToSend6(id, if_id, ip, pkt_id) => {
                let timestamp = current_time + self.cfg.topo.get_delay_v6(id, if_id, &ip).unwrap();
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

    /**
     * Part of a timespot where the delayed send are actually sent
     */
    fn sending_time_loop(&self, ta: TimestampActions) -> Result<()> {
        for (process, pkt_id) in ta.flatten() {
            //println!("Process {:?} should send packet {:?}", process, pkt_id);
            let msg = serialize_rust(Message::Send(pkt_id));
            //println!("{} Send({})", process, pkt_id);
            self.qs[process as usize].send(1, &msg.buffer)?;

            let mut msg: Buffer = Buffer::new();
            if let Ok(_) = self.qs[0].recv(&mut msg.buffer) {
                if let Message::Sent(p, p_id) = deserialize_rust(msg) {
                    if p != process || p_id != pkt_id {
                        eprintln!(
                            "bad Sent received:\n\texpected: Sent({},{})\n\treceived: Sent({},{})",
                            process, pkt_id, p, p_id
                        );
                        panic!("error");
                    }
                    //println!("{} Sent({})", p, p_id);
                } else {
                    eprintln!(
                        "bad message received:\n\texpected: Sent({},{})\n\treceived: {:?}",
                        process,
                        pkt_id,
                        deserialize_rust(msg)
                    );
                    panic!("error");
                }
            } else {
                eprintln!("could not receive Sent");
                panic!("error");
            }
        }
        Ok(())
    }

    /**
     * Part of a timespot where the processes actually run
     */
    fn running_time_loop(&mut self, current_time: u64) -> Result<State> {
        let mut msg: Buffer = Buffer::new();
        loop {
            //println!("Running time loop");
            match self.qs[0].recv(&mut msg.buffer) {
                Ok(_) => {
                    //println!("Leader received {:?}", Into::<Message>::into(msg));
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
                    //println!("nb-finished: {}, nf_blocked: {}, nb_follower: {}", nb_finished, nb_blocked, self.cfg.nb_follower);
                    if nb_finished == self.cfg.nb_follower {
                        return Ok(State::Finished);
                    }
                    if nb_blocked + nb_finished == self.cfg.nb_follower {
                        //all followers are blocked or finished, we need to make a step in time
                        return Ok(State::Blocked);
                    }
                }
                Err(e) => {
                    eprintln!("Message error: {e}");
                    panic!("recv on leader queue failed");
                }
            }
        }
    }

    /**
     * Main loop
     */
    fn main_loop(&mut self) -> Result<u8> {
        //println!("Reaching main loop");
        let msg = serialize_rust(Message::WakeUp(0));
        for (s, q) in self.states.iter_mut().zip(self.qs[1..].iter()) {
            //println!("(s, q): {:?}", (&s, q));
            q.send(1, &msg.buffer)?;
            //println!("sent {:?}", msg.buffer);
            *s = State::Running;
        }
        //println!("Before if");
        if let Ok(State::Finished) = self.running_time_loop(0) {
            eprintln!("Simulation finished by all process finishing");
            return Ok(0);
        }
        //println!("After if");
        loop {
            //println!("Events: {:?}", self.events);
            if let Some((time, ta)) = self.events.pop_first() {
                /*************** First half, sending time ***************/
                let _ = self.sending_time_loop(ta);

                /************** Second half, running time ***************/
                let msg = serialize_rust(Message::WakeUp(time));
                for (s, q) in self.states.iter_mut().zip(self.qs[1..].iter()) {
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
                eprintln!("Simulation finished by all process beeing blocked with no more events");
                return Ok(1);
            }
        }
    }

    fn run(mut self) {
        /*unsafe {
            let c_str = CString::new(format!("{}_{}", &self.cfg._qname, 0)).unwrap();
            println!("c_str: {:?}", c_str);
            println!("_qo: {:?}", libc::mq_open(c_str.as_ptr() as *const i8, libc::O_WRONLY));
            libc::perror(c_str.as_ptr() as *const i8);
        };*/
        self.leader_init_queues()
            .expect("leader queues initialisation failed"); //open the communication queues
        for i in 0..self.cfg.nb_follower {
            //start the followers
            //println!("{:?} {:?}", (i+1) as u8, &[CString::new((format!("ID={}", i+1)).to_string().as_str()).unwrap().as_c_str(),
            //CString::new((format!("LD_PRELOAD={}", LIB_NAME)).to_string().as_str()).unwrap().as_c_str()]);
            self.run_follower(
                (i + 1) as u8,
                &[
                    CString::new((format!("ID={}", i + 1)).to_string().as_str())
                        .unwrap()
                        .as_c_str(),
                    CString::new((format!("LD_PRELOAD={}", LIB_NAME)).to_string().as_str())
                        .unwrap()
                        .as_c_str(),
                ],
            )
            .expect("run follower failed");
            // TODO: add in the env the name of the queue
            //std::thread::sleep(std::time::Duration::from_millis(5000));
        }

        self.main_loop().expect("main loop failed");
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        // no arguments passed
        1 => {
            eprintln!("Please provide a config file");
        }
        // one argument passed
        2 => {
            let sim = Simulation::new(
                Config::new(Path::new(&args[1])).expect("Error parsing config file"),
            );
            sim.run();
        }
        _ => {
            eprintln!("Please provide a config file");
        }
    }
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

        assert_eq!(deserialize_rust(serialize_rust(GetRand(42))), GetRand(42));
        assert_ne!(deserialize_rust(serialize_rust(GetRand(42))), GetRand(43));

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
            GetRand(42),
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
         * Return a star centred on 10.0.0.100
         */
        fn test_topo(nb: usize) -> Self {
            let mut nodes: String = String::from(
                "  node [
    id 100
    label \"Node 100\"
    interface [
      id 0
      label \"wlp4s0\"
      ip [
        type \"v4\"
        ip \"10.0.0.100\"
      ]
    ]
  ]
",
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
    target 100
    target_if 0
    label \"Link\"
    metric 20
    type \"symmetric\"
  ]
",
                    i
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
        fn test_config(nb: usize) -> Self {
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
                ),
                Process::new(
                    CString::from(EXE_NAMES[1]),
                    CString::from(EXE_NAMES[1]),
                    cstringify(&EXE2_ARGS),
                ),
            ];

            return Self {
                nb_follower: nb,
                _qname: QNAME.to_string(),
                exe: processes,
                random_number: RANDOM_NUMBER,
                topo: NetworkTopology::test_topo(nb),
            };
        }
    }

    /**
     * In a topology of NB_FOLLOWERS processes, nb_sender of them send messages that should be received at the same time.
     * This test checks that senders are woken up one at a time, following a deterministic order.
     */
    fn test_determinism_setup(nb_sender: usize) {
        //let mut already_received = false;

        for _iter in 0..2 {
            //Setting up the environment
            let mut sim = Simulation::new(Config::test_config(NB_FOLLOWERS!()));
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
                    HasToSend4(_, Ipv4AddrC::new(10, 0, 0, 100), pkt_id),
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
        test_determinism_setup(9);
    }
}
