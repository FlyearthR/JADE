use network_time_simulator::{Ipv4AddrC, Ipv6AddrC};
use std::fs;
use std::collections::{BTreeMap, HashMap};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::io::{Result, ErrorKind as IOErrorKind};
use toml::de::Error as TomlError;
use toml::Value;
use gml_parser::{Edge, GMLObject, GMLValue, Graph, HasGMLAttributes, ReadableGMLAttributes};


pub fn cstringify(arr: &[&CStr]) -> Vec<CString> {
    let mut ret: Vec<CString> = Vec::with_capacity(arr.len());
    for e in arr.iter() {
        ret.push(CString::from(*e));
    }
    return ret;
}

#[derive(Debug)]
pub struct NetworkTopology { 
    grf: Graph,
    ip4_node: HashMap<Ipv4AddrC, (u8, u8)>,
    ip6_node: HashMap<Ipv6AddrC, (u8, u8)>,
}

impl NetworkTopology {
    #[allow(dead_code)]
    pub fn load(graph: &str) -> Self {
        Self::from_graph(Graph::from_gml(GMLObject::from_str(graph).unwrap()).unwrap())
    }

    fn from_graph(grf: Graph) -> Self {
        let mut ip4_node: HashMap<Ipv4AddrC, (u8, u8)> = HashMap::new();
        let mut ip6_node: HashMap<Ipv6AddrC, (u8, u8)> = HashMap::new();

        for node in &grf.nodes {
            for att in node.attributes() {
                if att.0 == "interface" {
                    let GMLValue::GMLObject(ref interface) = att.1 else {
                        panic!("Failed to read the topology file: an interface should contains values")
                    };
                    let mut ip4s: Vec<Ipv4AddrC> = Vec::new();
                    let mut ip6s: Vec<Ipv6AddrC> = Vec::new();
                    let mut id = -1;
                    for if_att in &(*interface).pairs {
                        match (if_att.0.as_str(), &if_att.1) {
                            ("id", &GMLValue::GMLInt(ref if_id)) => id = if_id.clone(),
                            ("ip", &GMLValue::GMLObject(ref ip)) => {
                                let ip_att = &ip.pairs;
                                // handle symmetric links
                                if ip_att[0].1 == GMLValue::GMLString("v4".to_string()) {
                                    let GMLValue::GMLString(ref ip_addr) = ip_att[1].1 else {
                                        panic!("Error parsing GML: an IP should contain a field ip");
                                    };
                                    ip4s.push(ip_addr.into());
                                } else {
                                    let GMLValue::GMLString(ref ip_addr) = ip_att[1].1 else {
                                        panic!("Error parsing GML: an IP should contain a field ip");
                                    };
                                    ip6s.push(ip_addr.into());
                                }
                            },
                            ("label", _) => {},
                            a => panic!("Error parsing GML: unknown attribute in IP: {:?}", a)
                        }
                    }
                    if id == -1 {
                        panic!("Error parsing GML: an interface should contain an id")
                    }
                    for ipv4 in ip4s {
                        ip4_node.insert(ipv4, (node.id as u8, id as u8));
                    }
                    for ipv6 in ip6s {
                        ip6_node.insert(ipv6, (node.id as u8, id as u8));
                    }
                }
            }
        }
        return Self {
            grf,
            ip4_node,
            ip6_node,
        };
    }

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::load(r#"
graph [            
   id 4           
]"#)
    }

    fn match_peers(edge: &Edge, id_src: u8, id_if_src: u8, id_dst: u8, id_if_dst: u8) -> bool {
        let GMLValue::GMLInt(e_if_src) = edge.get_attribute("source_if").unwrap().1 else {panic!("No source interface for the edge {:?}", edge)};
        let GMLValue::GMLInt(e_if_dst) = edge.get_attribute("target_if").unwrap().1 else {panic!("No target interface for the edge {:?}", edge)};
        let GMLValue::GMLString(ref edge_type) = edge.get_attribute("type").unwrap().1 else {panic!("No type for the edge {:?}", edge)};
        match edge_type.as_str() {
            "symmetric" => {
                (edge.source == id_src as i64 && edge.target == id_dst as i64
                && e_if_src == id_if_src as i64 && e_if_dst == id_if_dst as i64)
                || (edge.source == id_dst as i64 && edge.target == id_src as i64
                && e_if_src == id_if_dst as i64 && e_if_dst == id_if_src as i64)
            },
            "directed" => edge.source == id_src as i64 && edge.target == id_dst as i64
                            && e_if_src == id_if_src as i64 && e_if_dst == id_if_dst as i64,
            t => panic!("Bad link type : {t}"),
        }

    }

    fn get_delay(&self, peer: (u8, u8), id_src: u8, id_if_src: u8) -> Option<u64> {
        self.grf.edges.iter()
            .find(|&edge| Self::match_peers(edge, id_src, id_if_src, peer.0, peer.1))
            .and_then(|edge| {
                let Some((_, GMLValue::GMLInt(metric))) = edge.get_attribute("metric")
                else {panic!("Error parsing link metric")};
                Some(*metric as u64)
            })
        
    }

    #[allow(dead_code)]
    pub fn get_delay_v4(&self, id_src: u8, id_if_src: u8, addr_dst: &Ipv4AddrC) -> Option<u64> {
        if let Some(peer) = self.ip4_node.get(addr_dst) {
            self.get_delay(*peer, id_src, id_if_src)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn get_delay_v6(&self, id_src: u8, id_if_src: u8, addr_dst: &Ipv6AddrC) -> Option<u64> {
        if let Some(peer) = self.ip6_node.get(addr_dst) {
            self.get_delay(*peer, id_src, id_if_src)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Process {
    pub name: CString,
    pub path: CString,
    pub args: Vec<CString>,
}

impl Process {
    fn empty_new() -> Self {
        Self {
            name: CString::from(c""),
            path: CString::from(c""),
            args: vec![],
        }
    }

    pub fn new(name: CString, path: CString, args: Vec<CString>) -> Self {
        Self {
            name,
            path,
            args,
        }
    }

    pub fn optionable_new(name: Option<&str>, path: Option<&str>, args: Option<&Vec<Value>>) -> Option<Self> {
        let mut ret = Self::empty_new();
        if let Some(n) = name {
            ret.name = CString::new(n).unwrap();
        } else {
            return None;
        }

        if let Some(p) = path {
            ret.path = CString::new(p).unwrap();
        } else {
            return None;
        }

        if let Some(a) = args {
            let args_vec = a.iter()
                .filter_map(|arg| arg.as_str().map(|s| CString::new(s).unwrap()))
                .collect();
            ret.args = args_vec;
        } else {
            ret.args = Vec::new();
        }
        return Some(ret);
    }
}

#[derive(Debug)]
pub struct Config { //TODO : here objet config
    pub nb_follower: usize,
    pub _qname: String,
    pub exe: Vec<Process>,
    pub random_number: u64,
    pub topo: NetworkTopology,
}

impl Config {
    #[allow(dead_code)]
    pub fn default_config() -> Self {
        const NB_FOLLOWER: usize = 2;
        const QNAME: &str = "/nts_test";
        const EXE_NAMES: [&CStr; 2] = [c"./client", c"./server"];
        const EXE1_ARGS: [&CStr; 5] = [c"./client", c"-i", c"127.0.0.1", c"-p", c"4443"];
        const EXE2_ARGS: [&CStr; 5] = [c"./server", c"-i", c"127.0.0.1", c"-p", c"4443"];
        const RANDOM_NUMBER: u64 = 84;
    
        return Self {
            nb_follower: NB_FOLLOWER,
            _qname: QNAME.to_string(),
            exe: vec![Process::new(CString::from(c"name"), CString::from(EXE_NAMES[0]),cstringify(&EXE1_ARGS)),
                        Process::new(CString::from(c"name"), CString::from(EXE_NAMES[1]), cstringify(&EXE2_ARGS))],
            random_number: RANDOM_NUMBER,
            topo: NetworkTopology::new(),
        }
    }

    fn from(content: String) -> std::result::Result<Config, TomlError> {
        let value: Value = toml::from_str(&content)?;

        let nb_follower:usize = value.get("nb_follower").and_then(Value::as_integer).unwrap_or(0) as usize;
        let _qname = value.get("qname").and_then(Value::as_str).unwrap_or("").to_string();

        let mut exe = Vec::new();

        if let Some(executables) = value.get("executables").and_then(Value::as_table) {
            if let Some(exes) = executables.get("exe").and_then(Value::as_array) {
                for e in exes {
                    if let Some(exe_table) = e.as_table() {
                        if let Some(e) = Process::optionable_new(exe_table.get("name").and_then(Value::as_str),
                                                                exe_table.get("path").and_then(Value::as_str),
                                                                exe_table.get("args").and_then(Value::as_array))
                        {
                            exe.push(e);
                        }
                    }
                }
            }
        }

        let random_number = value.get("random_number").and_then(Value::as_integer).unwrap_or(0) as u64;
        if let Some(topo_path) = value.get("graph").and_then(Value::as_str) {
            let topo =  NetworkTopology::from_graph(Graph::from_gml(
                                        GMLObject::from_str(
                                            &fs::read_to_string(Path::new(topo_path)).expect("Failed to read the topology file")
                                            ).unwrap())
                                        .unwrap());
            return Ok(Config {
                nb_follower,
                _qname,
                exe,
                random_number,
                topo,
            });
            
        }
        return Ok(Config {
            nb_follower,
            _qname,
            exe,
            random_number,
            topo: NetworkTopology::new(),
        })

    }

    pub fn new(path: &Path) -> std::result::Result<Config, TomlError> {
        let content = fs::read_to_string(path).expect("Failed to read the config file");

        Self::from(content)
        
    }

    /**
     * Deletes the queues created for the run
     * @return: Ok on success
     **/
    pub fn unlink_queues(&self) -> Result<()>{
        let mut ret = None;
        for i in 0..self.nb_follower+1 {
            match posixmq::remove_queue(&format!("{}_{}", self._qname, i)) {
                Err(e) => {
                    if e.kind() != IOErrorKind::NotFound {
                        eprintln!("Cannot remove queue {}: {}", i, e);
                        ret = Some(e);
                    }
                },
                _ => {
                    //All is ok
                }
            }
        }
        if let Some(e) = ret {
            Err(e)
        } else {
            Ok(())
        }
    }
}

/**
 * A TimestampActions gathers all the actions to perform at a specific timestpot.
 * @to_wake_up: the list of processes that have a known reason to be woken up a that timestamp
 * @has_to_send: a list of pairs (process that have something to send , packet IDs)
 */
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TimestampActions {
    to_wake_up: Vec<u8>,
    has_to_send: BTreeMap<u8, Vec<u64>>,
}

#[allow(dead_code)]
impl TimestampActions {
    
    /**
     * Create a new empty TimestampActions
     */
    pub fn new() -> Self {
        TimestampActions {
            to_wake_up: Vec::new(),
            has_to_send: BTreeMap::new()
        }
    }
    /**
     * Create a new TimestampActions with one process to wake up
     */
    pub fn new_process(id: u8) -> Self {
        TimestampActions {
            to_wake_up: vec![id],
            has_to_send: BTreeMap::new()
        }
    }

    /**
     * Create a new TimestampActions with one packet to send
     */
    pub fn new_pkt_id(id: u8, pkt_id: u64) -> Self {
        let mut t = TimestampActions {
            to_wake_up: Vec::new(),
            has_to_send: BTreeMap::new()
        };
        t.add_packet(id, pkt_id);
        return t;
    }

    /**
     * Adds a process occurence to wake up
     * Returns the number of process occurences
     */
    pub fn add_process(&mut self, id: u8) -> usize {
        self.to_wake_up.push(id);
        return self.to_wake_up.len();
    }

    /**
     * Deletes a process occurence to wake up
     * Returns the number of process occurences
     */
    pub fn del_process(&mut self, id: u8) -> usize {
        self.to_wake_up.remove(self.to_wake_up.iter().position(|x| *x == id).unwrap());
        return self.to_wake_up.len();
    }

    /**
     * Check if the process is contained in the TimestampActions
     */
    pub fn contain_process(&self, id: u8) -> bool {
        return self.to_wake_up.contains(&id);
    }

    /**
     * Returns the number of occurences of processes that have to be woken up
     */
    pub fn nb_process(&self) -> usize {
        return self.to_wake_up.len();
    }

    /**
     * Adds a packet to send
     */
    pub fn add_packet(&mut self, id: u8, pkt_id: u64) {
        if let Some(x) = self.has_to_send.get_mut(&id) {
            x.push(pkt_id);
        } else {
            self.has_to_send.insert(id, vec![pkt_id]);
        }
    }

    /**
     * Returns the number of *processes* that have packets to send
     */
    pub fn nb_pkt(&self) -> usize {
        return self.has_to_send.len();
    }

    /**
     * Returns a Vec of pairs (process_id, pkt_id)
     */
    pub fn flatten(self) -> Vec<(u8, u64)> {
        let mut ret: Vec<(u8, u64)> = Vec::new();
        for (process_id, pkt_ids) in self.has_to_send {
            for pkt_id in pkt_ids {
                ret.push((process_id, pkt_id))
            }
        }
        return ret;
    }
}


#[cfg(test)]
mod unit_testing {
    use super::{Config, TimestampActions};
    use std::ffi::CString;
    use super::cstringify;
    use std::path::Path;
    use network_time_simulator::Ipv4AddrC;

    #[test]
    fn simple_config() {
        let cfg = Config::new(Path::new("tests/mini_config.toml")).unwrap();
        assert_eq!(cfg._qname, "some_queue");
        assert_eq!(cfg.random_number, 123);
        assert_eq!(cfg.exe[0].name, CString::from(c"client"));
        assert_eq!(cfg.exe[0].path, CString::from(c"examples/miniP/client"));
        assert_eq!(cfg.exe[0].args, cstringify(&[c"./client", c"-i", c"127.0.0.1", c"-p", c"4443"]));
        assert_eq!(cfg.exe[1].name, CString::from(c"server"));
        assert_eq!(cfg.exe[1].path, CString::from(c"examples/miniP/server"));
        assert_eq!(cfg.exe[1].args, cstringify(&[c"./server", c"-i", c"127.0.0.1", c"-p", c"4443"]));
    }

    #[test]
    fn delayed_links() {
        let cfg = Config::new(Path::new("tests/linked_graph_config.toml")).unwrap();
        // First edge
        assert_eq!(cfg.topo.get_delay_v4(2, 0, &Ipv4AddrC::from("192.168.1.2")), Some(20));
        assert_eq!(cfg.topo.get_delay_v4(2, 0, &Ipv4AddrC::from("192.168.1.1")), Some(20));
        assert_eq!(cfg.topo.get_delay_v4(1, 0, &Ipv4AddrC::from("172.16.0.2")), Some(20));
        // Second edge
        assert_eq!(cfg.topo.get_delay_v4(1, 1, &Ipv4AddrC::from("172.16.0.2")), Some(10));
        assert_eq!(cfg.topo.get_delay_v4(2, 0, &Ipv4AddrC::from("10.0.0.1")), Some(10));

    }

    #[test]
    fn ta_test() {
        let mut ta_n = TimestampActions::new();
        assert!(!ta_n.contain_process(1));
        assert_eq!(ta_n.nb_process(), 0);
        assert_eq!(ta_n.nb_pkt(), 0);

        let mut ta_pc = TimestampActions::new_process(1);
        assert!(ta_pc.contain_process(1));
        assert_eq!(ta_pc.nb_process(), 1);
        assert_eq!(ta_pc.nb_pkt(), 0);

        let ta_pk = TimestampActions::new_pkt_id(2, 2);
        assert!(!ta_pk.contain_process(1));
        assert_eq!(ta_pk.nb_process(), 0);
        assert_eq!(ta_pk.nb_pkt(), 1);

        assert_eq!(ta_n.add_process(1), 1);
        assert_eq!(ta_n.nb_process(), 1);
        assert!(ta_n.contain_process(1));
        assert_eq!(ta_n, ta_pc);

        assert_eq!(ta_n.add_process(1), 2);
        assert_eq!(ta_n.nb_process(), 2);
        assert!(ta_n.contain_process(1));
        
        assert_eq!(ta_n.add_process(1), 3);
        assert_eq!(ta_n.nb_process(), 3);
        assert!(ta_n.contain_process(1));
        
        assert_eq!(ta_n.del_process(1), 2);
        assert_eq!(ta_n.nb_process(), 2);
        assert!(ta_n.contain_process(1));
        
        assert_eq!(ta_n.del_process(1), 1);
        assert_eq!(ta_n.nb_process(), 1);
        assert!(ta_n.contain_process(1));
        assert_eq!(ta_n, ta_pc);
        
        assert_eq!(ta_n.del_process(1), 0);
        assert_eq!(ta_n.nb_process(), 0);
        assert!(!ta_n.contain_process(1));
        
        ta_n.add_packet(2, 2);
        assert_eq!(ta_n, ta_pk);

        ta_pc.add_packet(3, 2);
        assert_ne!(ta_n, ta_pc);

        let ta_pc2 = TimestampActions::new_pkt_id(2, 3);
        assert_ne!(ta_n, ta_pc2);

        ta_pc.add_packet(2, 4);
        ta_pc.add_packet(2, 5);
        ta_pc.add_packet(3, 4);
        assert_eq!(ta_pc.nb_pkt(), 2);

        let mut ta_vec = ta_pc.flatten();
        
        assert!(ta_vec.contains(&(3,2)));
        let index = ta_vec.iter().position(|x| *x == (3,2)).unwrap();
        ta_vec.swap_remove(index);
        
        assert!(ta_vec.contains(&(2,4)));
        let index = ta_vec.iter().position(|x| *x == (2,4)).unwrap();
        ta_vec.swap_remove(index);
        
        assert!(ta_vec.contains(&(2,5)));
        let index = ta_vec.iter().position(|x| *x == (2,5)).unwrap();
        ta_vec.swap_remove(index);
        
        assert!(ta_vec.contains(&(3,4)));
        let index = ta_vec.iter().position(|x| *x == (3,4)).unwrap();
        ta_vec.swap_remove(index);
        assert_eq!(ta_vec.len(), 0);

    }
}
