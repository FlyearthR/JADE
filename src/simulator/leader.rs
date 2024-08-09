use network_time_simulator::SIZE_BUFFER;
use posixmq::PosixMq;
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
    Waiting,
    Blocked,
    ToWakeUp,
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
 * Initialize one queue on which the leader will wait for the messages from the followers and a
 * queue by follower
 * @arg nb: the number of follower
 * @return: on succes return a vector of posix queue, the first one beeing the leader's queue
 **/
fn leader_init_queues(nb: u8) -> Result<Vec<PosixMq>> {
    let mut qs = Vec::new();
    qs.push(posixmq::OpenOptions::readonly() //the leader will receive messages on this queue
            .max_msg_len(SIZE_BUFFER)
            .capacity(nb as usize)
            .create()
            .open(&format!("{}_{}", get_identifier(), 0))
            ?);

    for i in 0..nb {
        qs.push(posixmq::OpenOptions::writeonly() //the leader will send messages on those queues
                .max_msg_len(SIZE_BUFFER)
                .capacity(nb as usize)
                .create()
                .open(&format!("{}_{}", get_identifier(), i+1))
                ?);
    }
    Ok(qs)
}

/**
 * Open one queue on which the follower will send message to the leader and
 * one queue on which the follower will receive messages from the server
 * @arg id: the id of the follower, strictly positive
 * @arg nb: the size of the queue (use the number of followers)
 * @return: on success return a pair of posix queues (follower_receiving_queue, follower_sending_queue)
 */
fn follower_init_queues(id: u8, nb: u8) -> Result<(PosixMq, PosixMq)> {
    let qo = posixmq::OpenOptions::writeonly() //the follower will send messages to the
        .max_msg_len(SIZE_BUFFER)                           //leader on this queue
        .capacity(nb as usize)
        .create()
        .open(&format!("{}_{}", get_identifier(), 0))
        .expect("failed to open queue qo for a follower");
    qo.set_cloexec(false)?;

    let qi = posixmq::OpenOptions::readonly() //the follower will receive the messages of
        .max_msg_len(SIZE_BUFFER)                          //the leader on this queue
        .capacity(nb as usize)
        .create()    
        .open(&format!("{}_{}", get_identifier(), id))
        .expect("failed to open queue qi for a follower");
    qi.set_cloexec(false)?;
    return Ok((qi, qo));
}

/**
 * Deletes the queues created for the run
 * @arg nb: the number of queues to delete (i.e. the number of followers + 1)
 * @return: Ok on success
 **/
fn unlink_queues(nb: u8) -> Result<()>{
    for i in 0..nb {
        posixmq::remove_queue(&format!("{}_{}", get_identifier(), i))?;
    }
    Ok(())
}

fn decision_process(states: &mut [State], msg: Message) -> Result<()> {
    match msg {
        Message::Progressed(id) => {
            println!("Received Progressed from {}", id);
            //some progress happened, let's wake up every process (except id)
            //we could add context (e.g. on which file descriptor it is waiting) to guess which
            //process should be woke up
            //this would be easier if writing syscalls were also recorded
            let ids = (0..states.len()).filter(|&i| i != (id-1) as usize);
            for i in ids {
                if states[i] != State::Finished {
                    states[i] = State::ToWakeUp;
                }
            }
            states[(id-1) as usize] = State::Waiting;
            Ok(())
        },
        Message::Stuck(id) => {
            println!("Received Stuck from {}", id);
            //did not progress
            states[(id-1) as usize] = State::Blocked;
            let ids = (0..states.len()).filter(|&i| i != (id-1) as usize);
            for i in ids {
                println!("Test {i}");
                if states[i] == State::Waiting {
                    println!("{i} detected");
                    states[i] = State::ToWakeUp;
                }
            }
            Ok(())
        },
        _ => {
            //this should not happen
            Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Unexpected message")))
        },
    }
}


#[derive(Debug)]
pub struct Config {
    nb_follower: u8,
    _qname: String,
    exe_names: Vec<CString>,
    exe_args: Vec<Vec<CString>>,
    random_number: u64,
}

impl Config {
    #[allow(dead_code)]
    fn default_config() -> Self {
        const NB_FOLLOWER: u8 = 2;
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
        let nb_follower = value.get("nb_follower").and_then(Value::as_integer).unwrap_or(0) as u8;
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
}
pub struct Simulation { //TODO: should contain PosixMq's
    cfg: Config,
    states: Vec<State>,
    events: BTreeMap<u64, Vec<u8>>,
    final_code: u8,
}

impl Simulation {

    fn new (cfg: Config) -> Self {
        let nb_f = cfg.nb_follower as usize;
        Self { cfg,
            states: vec![State::Running; nb_f as usize],
            events: BTreeMap::new(),
            final_code: 0
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

    fn messages_handler(&mut self, message: &Buffer, qs: &Vec<PosixMq>) -> Result<()> {
        match deserialize(*message) {
            Message::AddStep(id, t) => {
                println!("{:?}", self.events);
                println!("Received AddStep {} from {}", t, id);
                if let Some(x) = self.events.get_mut(&t) {
                    println!("first case");
                    x.push(id);
                } else {
                    println!("other case");
                    self.events.insert(t, vec![id]);
                }
                println!("{:?}", self.events);
            },
            Message::DelStep(id, t) => {
                println!("Received DelStep {} from {}", t, id);
                if let Some(x) = self.events.get_mut(&t) {
                    for elem in x.iter_mut() {
                        if elem == &id {
                            *elem = 0;
                            break;
                        }
                    }
                }
            },
            Message::GetTime(id) => {
                println!("Received GetTime from {}", id);
                //return head key of the BTreeMap
                let msg = serialize(Message::WakeUp(*(self.events.first_key_value().unwrap().0)));
                qs[id as usize].send(2, &msg.buffer)?;
                println!("response WakeUp at {id}, {:?}", msg.buffer);
            },
            Message::GetRand(id) => {
                println!("Received GetRand from {}", id);
                let msg = serialize(Message::WakeUp(self.cfg.random_number));
                qs[id as usize].send(2, &msg.buffer)?;
                println!("response WakeUp at {id}, {:?}", msg.buffer);
            },
            Message::Finished(id) => {
                println!("Node {} has finished", id);
                self.states[(id-1) as usize] = State::Finished;
                // TODO: manage this
            },
            msg => {
                decision_process(&mut self.states[..], msg)?;
            },
        }
        Ok(())
    }

    fn main_loop(mut self, qs: Vec<PosixMq>) -> Result<Self> {
        let mut msg: Buffer = Buffer::new();
        self.events.insert(0, vec![0]);
        loop {
            match qs[0].recv(&mut msg.buffer) {
                Ok(_) => {
                    println!("States before messages_handler {:?}", self.states);
                    self.messages_handler(&msg, &qs)?;
                    println!("States after messages_handler {:?}", self.states);
                    let mut nb_blocked = 0;
                    let mut nb_finished = 0;
                    for (s, q) in self.states.iter_mut().zip(qs[1..].iter()) {
                        match s {
                            State::Blocked => nb_blocked += 1,
                            State::ToWakeUp => {
                                    let msg = serialize(Message::WakeUp(*(self.events.first_key_value().unwrap().0)));
                                    q.send(1, &msg.buffer)?;
                                    println!("response WakeUp at {:?}, {:?}", q, msg.buffer);
                                    *s = State::Running;
                            },
                            State::Finished => {
                                nb_finished += 1;
                                println!("nb_finished incremented");
                            },
                            _ => {},
                        }
                        
                    }
                    println!("number finished: {nb_finished}");
                    if nb_finished == self.cfg.nb_follower {
                        return Ok(self);
                    }
                    if nb_blocked+nb_finished == self.cfg.nb_follower { //all followers are blocked or finished, we need to make a step in time
                        let _ = self.events.pop_first();
                        if self.events.first_key_value() == None {
                            // the simulation is finished
                            println!("Simulation finished by all process beeing blocked with no more events");
                            self.final_code = 1;
                            return Ok(self);
                        }
                        let msg = serialize(Message::WakeUp(*(self.events.first_key_value().unwrap().0)));
                        for (s, q) in self.states.iter_mut().zip(qs[1..].iter()) {
                            if s != &State::Finished {
                                q.send(1, &msg.buffer)?;
                                println!("response WakeUp at {:?}, {:?}", q, msg.buffer);
                                *s = State::Running;
                            }
                        }
                    }

                    println!("States after everything {:?}", self.states);
                },
                Err(e) =>  {
                    eprintln!("Message error: {e}");
                    panic!("recv on leader queue failed");
                },
            }      
        }
    }


    fn run(mut self) {
        println!("Hello, world!");
        for i in 0..self.cfg.nb_follower { //start the followers
            self.run_follower(i+1, &[CString::new((format!("ID={}", i+1)).to_string().as_str()).unwrap().as_c_str(),
                CString::new((format!("LD_PRELOAD={}", LIB_NAME)).to_string().as_str()).unwrap().as_c_str()]).expect("run follower failed");
        }
        let qs = leader_init_queues(self.cfg.nb_follower).expect("init queue failed"); //open the communication queues
        /*for q in &qs[1..] {
            q.send(1, b"Born?").expect("send first message failed");
        }*/

        self = self.main_loop(qs).expect("main loop failed");

        std::thread::sleep(std::time::Duration::from_millis(500));
        unlink_queues(self.cfg.nb_follower).expect("unlink failed"); //delete all communication queues
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
    use posixmq::PosixMq;
    use std::io::Result;
    use serial_test::{serial, parallel};
    use std::collections::VecDeque;
    use std::thread::{self};

    use network_time_simulator::{*, Message::*};
    use crate::*;

    #[test]
    #[serial]
    fn test_init_queues() {
        let qs = leader_init_queues(3).expect("creating leader queues");
        let mut buf = vec![0; 100];
        assert_eq!(qs.len(), 4);
        for i in 1..4 {
            let qio = follower_init_queues(i, 1).expect("creating follower queues");
            qs[i as usize].send(SIZE_BUFFER as u32, b"Born?").expect("send first message failed");
            assert_eq!(qio.0.recv(&mut buf).unwrap(), (SIZE_BUFFER as u32, "Born?".len()));
            qio.1.send(SIZE_BUFFER as u32, b"Yes!").expect("send first message failed");
            assert_eq!(qs[0].recv(&mut buf).unwrap(), (SIZE_BUFFER as u32, "Yes!".len()));
        }
        unlink_queues(3).expect("error during unlinking");
    }

    #[test]
    #[parallel]
    fn test_serialize_deserialize() {
        assert_eq!(deserialize(serialize(Progressed(42))), Progressed(42));
        assert_eq!(deserialize(serialize(Stuck(42))), Stuck(42));
        assert_eq!(deserialize(serialize(AddStep(42, 43))), AddStep(42, 43));
        assert_eq!(deserialize(serialize(DelStep(42, 42))), DelStep(42, 42));
        assert_eq!(deserialize(serialize(GetTime(42))), GetTime(42));
        assert_eq!(deserialize(serialize(GetRand(42))), GetRand(42));
        assert_eq!(deserialize(serialize(WakeUp(42))), WakeUp(42));
        assert_eq!(deserialize(serialize(Finished(42))), Finished(42));
    }

    #[test]
    #[serial]
    fn test_serialize_deserialize_mq() {
        let _ = unlink_queues(2);
        let qs = leader_init_queues(1).expect("leader queues creating");
        let qio = follower_init_queues(1, 1).expect("follower queues creating");
        let mut msg: Buffer = Buffer::new();
        let msgs = [Progressed(42), Stuck(42), AddStep(42, 43), DelStep(42, 42), GetTime(42), GetRand(42), Finished(42)];

        for m in msgs {
            let check = m.clone();
            qio.1.send(SIZE_BUFFER as u32, &serialize(m).buffer).expect("send follower message failed");
            qs[0].recv(&mut msg.buffer).unwrap();
            assert_eq!(deserialize(msg), check);
        }
        qs[1].send(1, &serialize(WakeUp(42)).buffer).expect("send leader message failed");
        qio.0.recv(&mut msg.buffer).unwrap();
        assert_eq!(deserialize(msg), WakeUp(42));
        let _ = unlink_queues(2);
    }

    fn mini_setup(nb_add_step: u64, nb_stuck: u64, nb_recv: u64, return_value: u8) {
        let _ = unlink_queues(2);
        let qs = leader_init_queues(2).expect("leader queues creating");
        let qf1 = follower_init_queues(1, 2).expect("follower queues creating");
        let qf2 = follower_init_queues(2, 2).expect("follower queues creating");
        let t = thread::spawn(|| {Simulation::new(Config::default_config()).main_loop(qs)});
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
        if let Ok(nb) = t.join().unwrap() {
            let _ = unlink_queues(2);
            assert_eq!(nb.final_code, return_value);
        }
        else {
            let _ = unlink_queues(2);
            assert!(false);
        }
    }

    #[test]
    #[serial]
    fn direct_finished() {
        mini_setup(0, 0, 0, 0);
    }

    #[test]
    #[serial]
    fn direct_blocked() {
        mini_setup(0, 1, 0, 1);
    }

    #[test]
    #[serial]
    fn one_step() {
        mini_setup(1, 1, 1, 0);
    }

    #[test]
    #[serial]
    fn too_much_steps() {
        mini_setup(2, 1, 1, 0);
    }

    fn test_decision_process(id_to_wake_up: u8, first_progess: u8) -> Result<()> {
        let mut msg_queue: VecDeque<Message> = VecDeque::from([]);
        const SIZE_TEST: usize = 4;
        let mut states: [State; SIZE_TEST] = [State::Blocked; SIZE_TEST];

        decision_process(&mut states, Message::Progressed(first_progess))?;
        while states[id_to_wake_up as usize] != State::ToWakeUp {
            println!("States: {:?}", states);
            for i in 0..SIZE_TEST {
                if states[i] == State::ToWakeUp {
                    states[i] = State::Running;
                    msg_queue.push_back(Message::Stuck(i as u8 +1));
                }
            }
            if let Some(msg) = msg_queue.pop_front() {
                decision_process(&mut states, msg)?;
            } else {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("no more messages")));
            }
        }
        Ok(())
    }

    #[test]
    #[parallel]
    fn test_one_to_wake_up() {
        // tests if the decision process eventually wakes up every process
        const SIZE_TEST: u8 = 4;
        for id_progressing in 0..SIZE_TEST {
            for id_to_wake_up in 0..SIZE_TEST {
                println!("{id_progressing}, {id_to_wake_up}");
                match test_decision_process(id_to_wake_up, id_progressing+1) {
                    Ok(()) => assert!(true),
                    _ => assert!(false),
                }
            }
        }
    }

    #[test]
    #[parallel]
    fn test_messages_handler() {
        // Setting up the environment
        let mut followers_qs: Vec<PosixMq> = vec![];
        const SIZE_TEST: u8 = 4;
        let mut sim = Simulation::new(Config::default_config());
        sim.cfg.nb_follower = SIZE_TEST;
        //sim.qname = "nts_test_message_handler";
        for i in 0..sim.cfg.nb_follower { //start the followers
            if let Ok((qi, _)) = follower_init_queues(i, sim.cfg.nb_follower) {
                followers_qs.push(qi);
            }
        }
        let leader_qs = leader_init_queues(sim.cfg.nb_follower).unwrap();

        //test with one timestamp
        let _ = sim.messages_handler(&serialize(Message::AddStep(1, 1)), &leader_qs);
        let mut v = vec![1];
        assert_eq!(sim.events.first_key_value(), Some((&1, &v)));

        let _ = sim.messages_handler(&serialize(Message::AddStep(2, 1)), &leader_qs);
        v.push(2);
        if let Some((&1, v2)) = sim.events.first_key_value() {
            assert!(v2.contains(&1));
            assert!(v2.contains(&2));
        } else {assert!(false);}
        assert_eq!(sim.events.first_key_value(), Some((&1, &v))); //TODO: compare at a higher level, not the Vec's

        let _ = sim.messages_handler(&serialize(Message::AddStep(2, 1)), &leader_qs);
        v.push(2);
        assert_eq!(sim.events.first_key_value(), Some((&1, &v)));

        let _ = sim.messages_handler(&serialize(Message::DelStep(1, 1)), &leader_qs);
        v[0] = 0;
        assert_eq!(sim.events.first_key_value(), Some((&1, &v.to_vec())));
        
    }
}

#[cfg(test)]
mod determinism {
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
        ($s:ident, $qfs:ident, $msgs:ident, $prioritized:ident, $tab:ident) => {
            {
                for i in 0..NB_FOLLOWERS!() {
                    let q: &Option<(PosixMq, PosixMq)> = &$qfs[i as usize];
                    $tab[i] = Some($s.spawn(move || {
                            match q.as_ref().unwrap().0.recv_timeout(&mut $msgs[i as usize].buffer, Duration::from_secs(1)) {
                                Ok(x) => {
                                    if i as usize > $prioritized {
                                        assert!(false);
                                    }
                                    Ok(x)
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
        ($ts:ident, $id_order:expr, DelStep(_, $step:expr), $qfs:ident) => {
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
            m_send!(DelStep(id_received+1, $step), $qfs); // sends a DelStep to the leader to acknoledge the packet has been sent
        };
    }

    macro_rules! m_send_1arg {
        ($msg:expr, $qfs:ident, $nb:expr) => {
            for i in 0..NB_FOLLOWERS!() {
                $qfs[i].as_ref().unwrap().1.send(1, &serialize($msg((i+1) as u8)).buffer).expect("send message failed");
            }
        }
    }
    macro_rules! m_send_2arg {
        ($msg:expr, $arg:expr, $qfs:ident, $nb:expr) => {
            for i in 0..$nb {
                $qfs[i].as_ref().unwrap().1.send(1, &serialize($msg((i+1) as u8, $arg)).buffer).expect("send message failed");
                println!("sent {:?} to {}", &serialize($msg((i+1) as u8, $arg)).buffer, i);
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
        }
    }

    impl Config {    
        fn test_config(nb: u8) -> Self {
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
        let _ = unlink_queues(NB_FOLLOWERS!() as u8);
        let mut msgs: [Buffer; NB_FOLLOWERS!() as usize] = [Buffer::new() ; NB_FOLLOWERS!() as usize];
        let nb_f = NB_FOLLOWERS!();
        let clo = |mut i| -> (PosixMq, PosixMq) {i += 1; follower_init_queues(i as u8, NB_FOLLOWERS!() as u8).expect("follower queues creating")};
        let qfs: [Option<(PosixMq, PosixMq)> ; NB_FOLLOWERS!()] = custom_arr_tpm(clo);
        //let mut already_received = false;
        
        for _iter in 0..2 {
            thread::scope(|sc| {
                let qs = leader_init_queues(NB_FOLLOWERS!() as u8).expect("leader queues creating");
                let t = sc.spawn(|| {Simulation::new(Config::test_config(NB_FOLLOWERS!())).main_loop(qs)});
                let mut receiving_order: [usize ; NB_FOLLOWERS!()] = [0 ; NB_FOLLOWERS!()];
                
                m_send!(AddStep(_, 1), qfs, nb_sender);
                m_send!(Stuck(_), qfs, nb_f);
                
                for i in 0..nb_sender {
                    thread::scope(|s| {
                        let mut ts: [Option<ScopedJoinHandle<Result<(u32, usize)>>> ; NB_FOLLOWERS!()] = [NONE_THREAD; NB_FOLLOWERS!()];
                        m_receive!(s, qfs, msgs, nb_sender, ts);
                        println!("\n\n\n\n\n\n\ni in nb_sender: {i}\niter: {_iter}\n\n\n\n\n\n\n");
                        m_check!(ts, receiving_order[i], DelStep(_, 1), qfs);
                    });
                }

                thread::scope(|s| {
                    let mut ts: [Option<ScopedJoinHandle<Result<(u32, usize)>>> ; NB_FOLLOWERS!()] = [NONE_THREAD; NB_FOLLOWERS!()];
                    m_receive!(s, qfs, msgs, nb_f, ts);
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
    fn test_determinism() {
        test_determinism_setup(2);
    }
}