use network_time_simulator::SIZE_BUFFER;
use posixmq::PosixMq;
use std::io::ErrorKind as IOErrorKind;
use std::path::Path;
use std::io::Result;
use std::fs;
use std::env;
use std::ffi::CStr;
use std::ffi::CString;
use std::collections::BTreeMap;
use fork::{fork, Fork};
use nix::unistd::execve;
use network_time_simulator::Message;
use network_time_simulator::serialize;
use network_time_simulator::deserialize;
use network_time_simulator::Buffer;
use toml::Value;
use toml::de::Error as TomlError;

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

fn cstringify(arr: &[&CStr]) -> Vec<CString> {
    let mut ret: Vec<CString> = Vec::with_capacity(arr.len());
    for e in arr.iter() {
        ret.push(CString::from(*e));
    }
    return ret;
}


const LIB_NAME: &str = "syscalls.so";

/**
 * Provide a unique (system-wide) identifier for message queues
 */
fn get_identifier() -> String {
    return "/nts_mq".to_string();
}

/**
 * Open one queue on which the follower will send message to the leader and
 * one queue on which the follower will receive messages from the leader
 * @arg id: the id of the follower, strictly positive
 * @arg nb: the size of the queue (use the number of followers)
 * @return: on success return a pair of posix queues (follower_receiving_queue, follower_sending_queue)
 */
fn follower_init_queues(id: u8, nb: usize) -> Result<(PosixMq, PosixMq)> {
    let qo = posixmq::OpenOptions::writeonly() //the follower will send messages to the
        .max_msg_len(SIZE_BUFFER)                           //leader on this queue
        .capacity(nb)
        .create()
        .open(&format!("{}_{}", get_identifier(), 0))
        .expect("failed to open queue qo for a follower");
    qo.set_cloexec(false)?;

    let qi = posixmq::OpenOptions::readonly() //the follower will receive the messages of
        .max_msg_len(SIZE_BUFFER)                          //the leader on this queue
        .capacity(nb)
        .create()    
        .open(&format!("{}_{}", get_identifier(), id))
        .expect("failed to open queue qi for a follower");
    qi.set_cloexec(false)?;
    return Ok((qi, qo));
}

#[derive(Debug)]
pub struct Config {
    nb_follower: usize,
    _qname: String,
    exe_names: Vec<CString>,
    exe_args: Vec<Vec<CString>>,
    random_number: u64,
}

impl Config {
    #[allow(dead_code)]
    fn default_config() -> Self {
        const NB_FOLLOWER: usize = 2;
        const QNAME: &str = "/nts_mq";
        const EXE_NAMES: [&CStr; 2] = [c"./client", c"./server"];
        const EXE1_ARGS: [&CStr; 5] = [c"./client", c"-i", c"127.0.0.1", c"-p", c"4443"];
        const EXE2_ARGS: [&CStr; 5] = [c"./server", c"-i", c"127.0.0.1", c"-p", c"4443"];
        const RANDOM_NUMBER: u64 = 84;
    
        return Self {
            nb_follower: NB_FOLLOWER,
            _qname: QNAME.to_string(),
            exe_names: cstringify(&EXE_NAMES),
            exe_args: vec![cstringify(&EXE1_ARGS), cstringify(&EXE2_ARGS)],
            random_number: RANDOM_NUMBER,
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

        let mut exe_names = Vec::new();
        let mut exe_args = Vec::new();

        if let Some(executables) = value.get("executables").and_then(Value::as_table) {
            if let Some(exes) = executables.get("exe").and_then(Value::as_array) {
                for exe in exes {
                    if let Some(exe_table) = exe.as_table() {
                        if let Some(path) = exe_table.get("path").and_then(Value::as_str) {
                            exe_names.push(CString::new(path).unwrap());
                        }

                        if let Some(args) = exe_table.get("args").and_then(Value::as_array) {
                            let args_vec = args.iter()
                                .filter_map(|arg| arg.as_str().map(|s| CString::new(s).unwrap()))
                                .collect();
                            exe_args.push(args_vec);
                        } else {
                            exe_args.push(Vec::new());
                        }
                    }
                }
            }
        }

        let random_number = value.get("random_number").and_then(Value::as_integer).unwrap_or(0) as u64;

        Ok(Config {
            nb_follower,
            _qname,
            exe_names,
            exe_args,
            random_number,
        })
    }

    /**
     * Deletes the queues created for the run
     * @return: Ok on success
     **/
    fn unlink_queues(&self) -> Result<()>{
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
    fn new() -> Self {
        TimestampActions {
            to_wake_up: Vec::new(),
            has_to_send: BTreeMap::new()
        }
    }
    /**
     * Create a new TimestampActions with one process to wake up
     */
    fn new_process(id: u8) -> Self {
        TimestampActions {
            to_wake_up: vec![id],
            has_to_send: BTreeMap::new()
        }
    }

    /**
     * Create a new TimestampActions with one packet to send
     */
    fn new_pkt_id(id: u8, pkt_id: u64) -> Self {
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
    fn add_process(&mut self, id: u8) -> usize {
        self.to_wake_up.push(id);
        return self.to_wake_up.len();
    }

    /**
     * Deletes a process occurence to wake up
     * Returns the number of process occurences
     */
    fn del_process(&mut self, id: u8) -> usize {
        self.to_wake_up.remove(self.to_wake_up.iter().position(|x| *x == id).unwrap());
        return self.to_wake_up.len();
    }

    /**
     * Returns the number of occurences of processes that have to be woken up
     */
    fn nb_process(&self) -> usize {
        return self.to_wake_up.len();
    }

    /**
     * Adds a packet to send
     */
    fn add_packet(&mut self, id: u8, pkt_id: u64) {
        if let Some(x) = self.has_to_send.get_mut(&id) {
            x.push(pkt_id);
        } else {
            self.has_to_send.insert(id, vec![pkt_id]);
        }
    }

    /**
     * Returns the number of *processes* that have packets to send
     */
    fn nb_pkt(self) -> usize {
        return self.has_to_send.len();
    }
}

pub struct Simulation {
    cfg: Config,
    states: Vec<State>,
    events: BTreeMap<u64, TimestampActions>,
    qs: Vec<PosixMq>,
}

impl Drop for Simulation {
    fn drop(&mut self) {
        let _ = self.cfg.unlink_queues();
    }
}

impl Simulation {

    fn new (cfg: Config) -> Self {
        let nb_f = cfg.nb_follower;
        let _ = cfg.unlink_queues();
        Self { cfg,
            states: vec![State::Running; nb_f],
            events: BTreeMap::new(),
            qs: Vec::with_capacity(nb_f+1)
        }
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
        println!("Lauching {id}: {:?} {:?} {:?}", env, self.cfg.exe_names[(id-1) as usize],
            &self.cfg.exe_args[(id-1) as usize]);
        match fork() {
            Ok(Fork::Parent(child)) => {
                println!("Child: {}", child);
                Ok(child)
            },
            Ok(Fork::Child) => {
                let _ = follower_init_queues(id, self.cfg.nb_follower);
                execve(&self.cfg.exe_names[(id-1) as usize], &self.cfg.exe_args[(id-1) as usize], env)?;
                Ok(0)
            }
            Err(_) => Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Fork failed"))),
        } 
    }

    /**
     * Initialize one queue on which the leader will wait for the messages from the followers and a
     * queue by follower
     * @arg nb: the number of follower
     * @return: returns a Result
     **/
    fn leader_init_queues(&mut self) -> Result<()> {
        self.qs.push( posixmq::OpenOptions::readonly() //the leader will receive messages on this queue
                .max_msg_len(SIZE_BUFFER)
                .capacity(self.cfg.nb_follower)
                .create()
                .open(&format!("{}_{}", get_identifier(), 0))
                ?);

        for i in 0..self.cfg.nb_follower {
            self.qs.push(posixmq::OpenOptions::writeonly() //the leader will send messages on those queues
                    .max_msg_len(SIZE_BUFFER)
                    .capacity(self.cfg.nb_follower)
                    .create()
                    .open(&format!("{}_{}", get_identifier(), i+1))
                    ?);
        }
        Ok(())
    }

    fn messages_handler(&mut self, message: &Buffer) -> Result<()> {
        match deserialize(*message) {
            Message::AddStep(id, t) => {
                println!("{:?}", self.events);
                println!("Received AddStep {} from {}", t, id);
                if let Some(x) = self.events.get_mut(&t) {
                    println!("first case");
                    x.add_process(id);
                } else {
                    println!("other case");
                    self.events.insert(t, TimestampActions::new_process(id));
                }
                println!("{:?}", self.events);
            },
            Message::DelStep(id, t) => {
                println!("Received DelStep {} from {}", t, id);
                if let Some(x) = self.events.get_mut(&t) {
                    x.del_process(id);
                }
            },
            Message::GetTime(id) => {
                println!("Received GetTime from {}", id);
                //return head key of the BTreeMap
                let msg = serialize(Message::WakeUp(*(self.events.first_key_value().unwrap().0)));
                self.qs[id as usize].send(2, &msg.buffer)?;
                println!("response WakeUp at {id}, {:?}", msg.buffer);
            },
            Message::GetRand(id) => {
                println!("Received GetRand from {}", id);
                let msg = serialize(Message::WakeUp(self.cfg.random_number));
                self.qs[id as usize].send(2, &msg.buffer)?;
                println!("response WakeUp at {id}, {:?}", msg.buffer);
            },
            Message::Finished(id) => {
                println!("Node {} has finished", id);
                self.states[(id-1) as usize] = State::Finished;
            },
            Message::Stuck(id) => {
                self.states[(id-1) as usize] = State::Blocked;
            },
            _ => {
                //TODO
            }
        }
        Ok(())
    }

    /**
     * Part of a timespot where the delayed send are actually sent
     */
    fn sending_time_loop(&mut self) -> Result<()> {
        Ok(())
    }

    /**
     * Part of a timespot where the processes actually run
     * TODO: parse HasToSendX
     */
    fn running_time_loop(&mut self) -> Result<State> {
        let mut msg: Buffer = Buffer::new();
        self.events.insert(0, TimestampActions::new());
        loop {
            match self.qs[0].recv(&mut msg.buffer) {
                Ok(_) => {
                    self.messages_handler(&msg)?;
                    let mut nb_blocked = 0;
                    let mut nb_finished = 0;
                    for s in self.states.iter_mut() {
                        match s {
                            State::Blocked => nb_blocked += 1,
                            State::Finished => nb_finished += 1,
                            _ => {},
                        }
                        
                    }
                    println!("number finished: {nb_finished}");
                    if nb_finished == self.cfg.nb_follower {
                        return Ok(State::Finished);
                    }
                    if nb_blocked+nb_finished == self.cfg.nb_follower { //all followers are blocked or finished, we need to make a step in time
                        return Ok(State::Blocked);
                    }

                },
                Err(e) =>  {
                    eprintln!("Message error: {e}");
                    panic!("recv on leader queue failed");
                },
            }      
        }
    }

    /**
     * Main loop
     */
    fn main_loop(&mut self) -> Result<u8> {
        loop {
            /*************** First half, sending time ***************/
            let _ = self.sending_time_loop();

            /************** Second half, running time ***************/
            let msg = serialize(Message::WakeUp(*(self.events.first_key_value().unwrap().0)));
            for (s, q) in self.states.iter_mut().zip(self.qs[1..].iter()) {
                if s != &State::Finished {
                    q.send(1, &msg.buffer)?;
                    *s = State::Running;
                }
            }
            if let Ok(State::Finished) = self.running_time_loop() {
                println!("Simulation finished by all process finishing");
                return Ok(0);
            }

            /************** Jumping to next timestamp ***************/
            let _ = self.events.pop_first();
            if self.events.first_key_value() == None {
                // the simulation is finished
                println!("Simulation finished by all process beeing blocked with no more events");
                return Ok(1);
            }
        }
    }

    fn run(mut self) {
        println!("Hello, world!");
        for i in 0..self.cfg.nb_follower { //start the followers
            self.run_follower((i+1) as u8, &[CString::new((format!("ID={}", i+1)).to_string().as_str()).unwrap().as_c_str(),
                CString::new((format!("LD_PRELOAD={}", LIB_NAME)).to_string().as_str()).unwrap().as_c_str()]).expect("run follower failed");
        }
        self.leader_init_queues().expect("leader queues initialisation failed"); //open the communication queues

        self.main_loop().expect("main loop failed");

        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        // no arguments passed
        1 => {
            println!("Please provide a config file");
        },
        // one argument passed
        2 => {
            let sim = Simulation::new(Config::new(Path::new(&args[1])).expect("Error parsing config file"));
            println!("{:?}", sim.cfg);
            sim.run();
        },
        _ => {
            println!("Please provide a config file");
        }
    }

}


#[cfg(test)]
mod unit_testing {
    use ntest::timeout;
    use posixmq::PosixMq;
    use serial_test::{serial, parallel};
    use std::thread::{self};
    use std::time::Duration;

    use network_time_simulator::{*, Message::*};
    use crate::*;

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
        assert_eq!(sim.qs.len(), NB_QUEUES_TEST+1);
        for i in 1..NB_QUEUES_TEST+1 {
            let qio = follower_init_queues(i as u8, NB_QUEUES_TEST).expect("creating follower queues");
            sim.qs[i].send(0 as u32, b"Born?").expect("send first message failed");
            assert_eq!(qio.0.recv_timeout(&mut buf, Duration::from_secs(1)).unwrap(), (0 as u32, "Born?".len()));
            qio.1.send(0 as u32, b"Yes!").expect("send first message failed");
            assert_eq!(sim.qs[0].recv_timeout(&mut buf, Duration::from_secs(1)).unwrap(), (0 as u32, "Yes!".len()));
        }
    }

    #[test]
    #[timeout(10000)]
    #[parallel]
    fn test_serialize_deserialize() {
        assert_eq!(deserialize(serialize(Stuck(42))), Stuck(42));
        assert_ne!(deserialize(serialize(Stuck(42))), Stuck(43));
        
        assert_eq!(deserialize(serialize(AddStep(42, 43))), AddStep(42, 43));
        assert_ne!(deserialize(serialize(AddStep(42, 43))), AddStep(42, 44));
        assert_ne!(deserialize(serialize(AddStep(42, 43))), AddStep(43, 43));
        
        assert_eq!(deserialize(serialize(DelStep(42, 42))), DelStep(42, 42));
        assert_ne!(deserialize(serialize(DelStep(42, 42))), DelStep(42, 43));
        assert_ne!(deserialize(serialize(DelStep(42, 42))), DelStep(43, 42));
        
        assert_eq!(deserialize(serialize(HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 45), 42))),
            HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 45), 42));
        assert_ne!(deserialize(serialize(HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 45), 42))),
            HasToSend4(43, Ipv4AddrC::new(42, 43, 44, 45), 42));
        assert_ne!(deserialize(serialize(HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 45), 42))),
            HasToSend4(42, Ipv4AddrC::new(43, 43, 44, 45), 42));
        assert_ne!(deserialize(serialize(HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 45), 42))),
            HasToSend4(42, Ipv4AddrC::new(42, 44, 44, 45), 42));
        assert_ne!(deserialize(serialize(HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 45), 42))),
            HasToSend4(42, Ipv4AddrC::new(42, 43, 45, 45), 42));
        assert_ne!(deserialize(serialize(HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 45), 42))),
            HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 46), 42));
        assert_ne!(deserialize(serialize(HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 45), 42))),
            HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 45), 43));
        
        assert_eq!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 43, 44, 46, 47, 48, 49), 42))),
            HasToSend6(42, Ipv6AddrC::new(42, 43, 43, 44, 46, 47, 48, 49), 42));

        assert_ne!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42))),
            HasToSend6(43, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42));

        assert_ne!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42))),
            HasToSend6(42, Ipv6AddrC::new(43, 43, 44, 45, 46, 47, 48, 49), 42));
        assert_ne!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42))),
            HasToSend6(42, Ipv6AddrC::new(42, 44, 44, 45, 46, 47, 48, 49), 42));
        assert_ne!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42))),
            HasToSend6(42, Ipv6AddrC::new(42, 43, 45, 45, 46, 47, 48, 49), 42));
        assert_ne!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42))),
            HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 46, 46, 47, 48, 49), 42));
        assert_ne!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42))),
            HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 47, 47, 48, 49), 42));
        assert_ne!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42))),
            HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 48, 48, 49), 42));
        assert_ne!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42))),
            HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 49, 49), 42));
        assert_ne!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42))),
            HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 50), 42));

        assert_ne!(deserialize(serialize(HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42))),
            HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 43));

        assert_eq!(deserialize(serialize(Send(42))), Send(42));
        assert_ne!(deserialize(serialize(Send(42))), Send(43));

        assert_eq!(deserialize(serialize(Sent(1, 42))), Sent(1, 42));
        assert_ne!(deserialize(serialize(Sent(1, 42))), Sent(1, 43));
        assert_ne!(deserialize(serialize(Sent(1, 42))), Sent(2, 42));

        assert_eq!(deserialize(serialize(GetTime(42))), GetTime(42));
        assert_ne!(deserialize(serialize(GetTime(42))), GetTime(43));
        
        assert_eq!(deserialize(serialize(GetRand(42))), GetRand(42));
        assert_ne!(deserialize(serialize(GetRand(42))), GetRand(43));
        
        assert_eq!(deserialize(serialize(WakeUp(42))), WakeUp(42));
        assert_ne!(deserialize(serialize(WakeUp(42))), WakeUp(43));
        
        assert_eq!(deserialize(serialize(Finished(42))), Finished(42));
        assert_ne!(deserialize(serialize(Finished(42))), Finished(43));
    }

    #[test]
    #[timeout(10000)]
    #[serial]
    fn test_serialize_deserialize_mq() {
        let mut sim = Simulation::new(Config::default_config_nb(1));
        sim.leader_init_queues().expect("leader queues creating");
        let qio = follower_init_queues(1, 1).expect("follower queues creating");
        let mut msg: Buffer = Buffer::new();
        let msgs = [Stuck(42), AddStep(42, 43),
            HasToSend4(42, Ipv4AddrC::new(42, 43, 44, 45), 42), HasToSend6(42, Ipv6AddrC::new(42, 43, 44, 45, 46, 47, 48, 49), 42),
            Send(42), Sent(1, 42), DelStep(42, 42), GetTime(42), GetRand(42), Finished(42)];

        for m in msgs {
            let check = m.clone();
            qio.1.send(SIZE_BUFFER as u32, &serialize(m).buffer).expect("send follower message failed");
            sim.qs[0].recv(&mut msg.buffer).unwrap();
            assert_eq!(deserialize(msg), check);
        }
        sim.qs[1].send(1, &serialize(WakeUp(42)).buffer).expect("send leader message failed");
        qio.0.recv(&mut msg.buffer).unwrap();
        assert_eq!(deserialize(msg), WakeUp(42));
    }

    fn mini_setup(nb_add_step: u64, nb_stuck: u64, nb_recv: u64, return_value: u8) {
        let mut sim = Simulation::new(Config::default_config());
        sim.leader_init_queues().expect("leader queues creating");
        let qf1 = follower_init_queues(1, 2).expect("creating follower queues");
        let qf2 = follower_init_queues(2, 2).expect("creating follower queues");
        let t = thread::spawn(move || {sim.main_loop()});
        for i in 1..nb_add_step+1 {
            println!("adding step");
            qf1.1.send(1, &serialize(AddStep(1, i)).buffer).expect("send add_step failed");
        }
        for _i in 1..nb_stuck+1 {
            println!("getting stuck");
            qf1.1.send(1, &serialize(Stuck(1)).buffer).expect("send stuck failed");
            qf2.1.send(1, &serialize(Stuck(2)).buffer).expect("send stuck failed");
        }

        for i in 1..nb_recv+1 {
            println!("receiving");
            let mut msg: Buffer = Buffer::new();
            qf1.0.recv(&mut msg.buffer).unwrap();
            assert_eq!(deserialize(msg), WakeUp(i));
            qf2.0.recv(&mut msg.buffer).unwrap();
            assert_eq!(deserialize(msg), WakeUp(i));
        }
        qf1.1.send(1, &serialize(Finished(1)).buffer).expect("send finished failed");
        qf2.1.send(1, &serialize(Finished(2)).buffer).expect("send finished failed");
        if let Ok(ret) = t.join().unwrap() {
            assert_eq!(ret, return_value);
        }
        else {
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
        for i in 0..sim.cfg.nb_follower { //start the followers
            if let Ok((qi, _)) = follower_init_queues(i as u8, sim.cfg.nb_follower) {
                followers_qs.push(qi);
            }
        }
        sim.leader_init_queues().unwrap();

        //test with one timestamp
        sim.messages_handler(&serialize(Message::AddStep(1, 1))).expect("message handler failing");
        let mut ta = TimestampActions{ to_wake_up: vec![1], has_to_send: BTreeMap::new()};
        assert_eq!(sim.events.first_key_value(), Some((&1, &ta)));

        let _ = sim.messages_handler(&serialize(Message::AddStep(2, 1)));
        ta.to_wake_up.push(2);
        if let Some((&1, v2)) = sim.events.first_key_value() {
            assert!(v2.to_wake_up.contains(&1));
            assert!(v2.to_wake_up.contains(&2));
        } else {assert!(false);}
        assert_eq!(sim.events.first_key_value(), Some((&1, &ta))); //TODO: compare at a higher level, not the Vec's

        let _ = sim.messages_handler(&serialize(Message::AddStep(2, 1)));
        ta.to_wake_up.push(2);
        assert_eq!(sim.events.first_key_value(), Some((&1, &ta)));

        let _ = sim.messages_handler(&serialize(Message::DelStep(1, 1)));
        ta.to_wake_up[0] = 0;
        assert_eq!(sim.events.first_key_value(), Some((&1, &ta)));
    }
}

#[cfg(test)]
mod determinism {
    use ntest::timeout;
    use posixmq::PosixMq;
    use std::io::Result;
    use std::thread::{self, ScopedJoinHandle};
    use std::time::Duration;
    use serial_test::serial;

    use network_time_simulator::{*, Message::*};
    use crate::*;

    macro_rules! NB_FOLLOWERS {
        () => {
            2 //must be root to get higher than 10
        };
    }

    const NONE_TPM: Option<(PosixMq, PosixMq)> = None;
    fn custom_arr_tpm(clo: impl Fn(u8) -> (PosixMq, PosixMq)) -> [Option<(PosixMq, PosixMq)>; NB_FOLLOWERS!()] {
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
                                                assert!(false);
                                            }
                                            Ok(x)
                                        },
                                        _ => {
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
     * Sends a DelStep to the leader to acknoledge the packet has been sent
     */
    macro_rules! m_check {
        ($ts:ident, $id_order:expr, Sent(_, $pkt_id:expr), $qfs:ident) => {
            let mut already_received = false;
            let mut id_received = usize::MAX;
            for i in 0..NB_FOLLOWERS!() {
                match $ts[i].take().expect("Uninit thread handle").join().unwrap() {
                    Ok((0,0)) => {
                        assert!(true); //all is ok
                        println!("i in Ok(0,0) NB_FOLLOWER: {i}");
                    },
                    Ok(_) => {
                        println!("i in Ok(_) NB_FOLLOWER: {i}");
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
                    Err(x) => {
                        println!("i in Err(x) NB_FOLLOWER: {i}");
                        println!("\n\n\n\n\n\n\nThread panic detected\n\n\n\n\n\n\n");
                        println!("{x}");
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
                $qfs[i].as_ref().unwrap().1.send(1, &serialize($msg((i+1) as u8)).buffer).expect("send message failed");
            }
        }
    }
    #[allow(unused_macros)]
    macro_rules! m_send_2arg {
        ($msg:expr, $arg:expr, $qfs:ident, $nb:expr) => {
            for i in 0..$nb {
                $qfs[i].as_ref().unwrap().1.send(1, &serialize($msg((i+1) as u8, $arg)).buffer).expect("send message failed");
                println!("sent {:?} to {}", &serialize($msg((i+1) as u8, $arg)).buffer, i);
            }
        }
    }
    macro_rules! m_send_3arg {
        ($msg:expr, $dest:expr, $pkt_id:expr, $qfs:ident, $nb:expr) => {
            for i in 0..$nb {
                $qfs[i].as_ref().unwrap().1.send(1, &serialize($msg((i+1) as u8, $dest, $pkt_id)).buffer).expect("send message failed");
                println!("sent {:?} to {}", &serialize($msg((i+1) as u8, $dest, $pkt_id)).buffer, i);
            }
        }
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
            $qfs[$id-1].as_ref().unwrap().1.send(1, &serialize(DelStep(($id) as u8, $step)).buffer).expect("send DelStep failed");
        };
        (DelStep(_, $step:expr), $qfs:ident, $nb:expr) => {
            m_send_2arg!(DelStep, $step, $qfs, $nb)
        };
        (AddStep(_, $step:expr), $qfs:ident, $nb:expr) => {
            m_send_2arg!(AddStep, $step, $qfs, $nb)
        };
        (HasToSend4(_, $dest:expr, $pkt_id:expr), $qfs:ident, $nb:expr) => {
            m_send_3arg!(HasToSend4, $dest, $pkt_id, $qfs, $nb)
        };
        (HasToSend6(_, $dest:expr, $pkt_id:expr), $qfs:ident, $nb:expr) => {
            m_send_3arg!(HasToSend6, $dest, $pkt_id, $qfs, $nb)
        };
        (Sent($id:expr, $pkt_id:expr), $qfs:ident) => {
            $qfs[$id-1].as_ref().unwrap().1.send(1, &serialize(Sent(($id) as u8, $pkt_id)).buffer).expect("send DelStep failed");
        };
    }

    impl Config {    
        fn test_config(nb: usize) -> Self {
            const QNAME: &str = "/nts_mq";
            const EXE_NAMES: [&CStr; 2] = [c"./client", c"./server"];
            const EXE1_ARGS: [&CStr; 5] = [c"./client", c"-i", c"127.0.0.1", c"-p", c"4443"];
            const EXE2_ARGS: [&CStr; 5] = [c"./server", c"-i", c"127.0.0.1", c"-p", c"4443"];
            const RANDOM_NUMBER: u64 = 84;
        
            return Self {
                nb_follower: nb,
                _qname: QNAME.to_string(),
                exe_names: cstringify(&EXE_NAMES),
                exe_args: vec![cstringify(&EXE1_ARGS), cstringify(&EXE2_ARGS)],
                random_number: RANDOM_NUMBER,
            }
        }
    }

    /**
     * In a topology of NB_FOLLOWERS processes, nb_sender of them send messages that should be received at the same time.
     * This test checks that senders are woken up one at a time, following a deterministic order.
     */
    fn test_determinism_setup(nb_sender:usize) {
        //Setting up the environment
        let mut sim = Simulation::new(Config::default_config());
        sim.cfg.nb_follower = NB_FOLLOWERS!();
        let mut msgs: [Buffer; NB_FOLLOWERS!() as usize] = [Buffer::new() ; NB_FOLLOWERS!()];
        let nb_f = NB_FOLLOWERS!();
        let clo = |mut i| -> (PosixMq, PosixMq) {i += 1; follower_init_queues(i as u8, NB_FOLLOWERS!()).expect("follower queues creating")};
        let qfs: [Option<(PosixMq, PosixMq)> ; NB_FOLLOWERS!()] = custom_arr_tpm(clo);
        let pkt_id = 1;
        //let mut already_received = false;
        
        for _iter in 0..2 {
            thread::scope(|sc| {
                sim.leader_init_queues().expect("leader queues creating");
                let t = sc.spawn(|| {Simulation::new(Config::test_config(NB_FOLLOWERS!())).main_loop()});
                let mut receiving_order: [usize ; NB_FOLLOWERS!()] = [0 ; NB_FOLLOWERS!()];
                
                m_send!(HasToSend4(_,Ipv4AddrC::new(1, 1, 1, 1), pkt_id), qfs, nb_sender);
                m_send!(Stuck(_), qfs, nb_f);
                
                for i in 0..nb_sender {
                    thread::scope(|s| {
                        let mut ts: [Option<ScopedJoinHandle<Result<(u32, usize)>>> ; NB_FOLLOWERS!()] = [NONE_THREAD; NB_FOLLOWERS!()];
                        m_receive!(s, qfs, msgs, nb_sender, ts, pkt_id);
                        println!("\n\n\n\n\n\n\ni in nb_sender: {i}\niter: {_iter}\n\n\n\n\n\n\n");
                        m_check!(ts, receiving_order[i], Sent(_, pkt_id), qfs);
                    });
                }

                thread::scope(|s| {
                    let mut ts: [Option<ScopedJoinHandle<Result<(u32, usize)>>> ; NB_FOLLOWERS!()] = [NONE_THREAD; NB_FOLLOWERS!()];
                    m_receive!(s, qfs, msgs, nb_f, ts, pkt_id);
                    for i in 0..nb_f {
                        let _ = ts[i].take().expect("uninit thread handle").join().unwrap();
                    }
                });
                m_send!(Finished(_), qfs, nb_f);
                let _ = t.join();
                let _ = sim.cfg.unlink_queues();
            });
        }
    }

    #[test]
    #[serial]
    #[timeout(10000)]
    fn test_determinism() {
        test_determinism_setup(2);
    }
}