use network_time_simulator::{Ipv4AddrC, Ipv6AddrC};
use std::fs;
use std::collections::{BTreeMap, HashMap};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::io::{Result, ErrorKind as IOErrorKind};
use toml::de::Error as TomlError;
use toml::Value;
use gml_parser::{GMLObject, Graph};


pub fn cstringify(arr: &[&CStr]) -> Vec<CString> {
    let mut ret: Vec<CString> = Vec::with_capacity(arr.len());
    for e in arr.iter() {
        ret.push(CString::from(*e));
    }
    return ret;
}

/**
 * Provide a unique (system-wide) identifier for message queues
 */
pub fn get_identifier() -> String {
    return "/nts_mq".to_string();
}

#[derive(Debug)]
pub struct NetworkTopology {
    grf: Graph,
    ip4_node: HashMap<Ipv4AddrC, u8>,
    ip6_node: HashMap<Ipv6AddrC, u8>,
}

impl NetworkTopology {
    #[allow(dead_code)]
    fn load(graph: &str) -> Self {
        Self::from_graph(Graph::from_gml(GMLObject::from_str(graph).unwrap()).unwrap())
    }

    fn from_graph(grf: Graph) -> Self {
        return Self {
            grf,
            ip4_node: HashMap::new(),
            ip6_node: HashMap::new(),
        };
    }

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::load(r#"
graph [            
   id 4           
]"#)
    }

    #[allow(dead_code)]
    pub fn get_delay_v4(&self, id_src: u8, addr_dst: &Ipv4AddrC) -> Option<u64> {
        if let Some(peer) = self.ip4_node.get(addr_dst) {
            Some(self.grf.edges.iter()
                .find(|&edge| edge.source == id_src as i64 && edge.target == *peer as i64)
                .and_then(|edge| edge.label.as_ref())?.parse::<u64>().unwrap())
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn get_delay_v6(&self, id_src: u8, addr_dst: &Ipv6AddrC) -> Option<u64> {
        if let Some(peer) = self.ip6_node.get(addr_dst) {
            Some(self.grf.edges.iter()
                .find(|&edge| edge.source == id_src as i64 && edge.target == *peer as i64)
                .and_then(|edge| edge.label.as_ref())?.parse::<u64>().unwrap())
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
pub struct Config {
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
        const QNAME: &str = "/nts_mq";
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

    pub fn new(path: &Path) -> std::result::Result<Config, TomlError> {
        // Read the file content
        let content = fs::read_to_string(path).expect("Failed to read the file");

        // Parse the content as a TOML Value
        let value: Value = toml::from_str(&content)?;

        // Extract the values from the TOML Value
        let nb_follower:usize = value.get("nb_follower").and_then(Value::as_integer).unwrap_or(0) as usize;
        let _qname = value.get("qname").and_then(Value::as_str).unwrap_or("").to_string();

        // TODO transformer tout ça en une seul struc qui représente un exécutable

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
        let topo = NetworkTopology::from_graph(Graph::from_gml(
            GMLObject::from_str(value.get("topology")
            .and_then(Value::as_str).unwrap_or("")).unwrap()).unwrap());
        Ok(Config {
            nb_follower,
            _qname,
            exe,
            random_number,
            topo,
        })
    }

    /**
     * Deletes the queues created for the run
     * @return: Ok on success
     **/
    pub fn unlink_queues(&self) -> Result<()>{
        let mut ret = None;
        for i in 0..self.nb_follower+1 {
            match posixmq::remove_queue(&format!("{}_{}", get_identifier(), i)) {
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
    pub fn nb_pkt(self) -> usize {
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

}