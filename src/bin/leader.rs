use posixmq::PosixMq;
use std::io::ErrorKind;
use std::io::Result;
use fork::{fork, Fork};
use nix::unistd::execve;
//use nix::sched::setns;
use std::ffi::CStr;
use std::ffi::CString;
use std::collections::BTreeMap;
use core::time::Duration;
use network_time_simulator::Message;
use network_time_simulator::serialize;
use network_time_simulator::deserialize;
use network_time_simulator::Buffer;

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

macro_rules! cstr {
    ( $s:literal ) => {{
        $crate::validate_cstr_contents($s.as_bytes());
        unsafe { std::mem::transmute::<_, &std::ffi::CStr>(concat!($s, "\0")) }
    }};
}

const NB_FOLLOWER: u8 = 2;
const QNAME: &str = "/nts_mq";
const EXE_NAMES: [&CStr; 2] = [cstr!("./client"), cstr!("./server")];
const EXE_ARGS: [[&CStr; 5]; 2] = [[cstr!("./client"), cstr!("-i"), cstr!("127.0.0.1"), cstr!("-p"), cstr!("4443")],
                                   [cstr!("./server"), cstr!("-i"), cstr!("127.0.0.1"), cstr!("-p"), cstr!("4443")]];
const LIB_NAME: &str = "/home/flyearth/1-PHD/Thesis/network_time_simulator/test/syscalls.so";
const RANDOM_NUMBER: u64 = 84;

/**
 * Initialize one queue on which the leader will wait for the messages from the followers and a
 * queue by follower
 * @arg nb: the number of follower
 * @return: on succes return a vector of posix queue, the first one beeing the leader's queue
 **/
fn init_queues(nb: u8) -> Result<Vec<PosixMq>> {
    let mut qs = Vec::new();
    qs.push(posixmq::OpenOptions::readonly() //the leader will receive messages on this queue
            .max_msg_len(10)
            .capacity(NB_FOLLOWER as usize)
            .create()
            .open(&format!("{}_{}", QNAME, 0))
            ?);

    for i in 0..nb {
        qs.push(posixmq::OpenOptions::writeonly() //the leader will send messages on those queues
                .max_msg_len(10)
                .capacity(NB_FOLLOWER as usize)
                .create()
                .open(&format!("{}_{}", QNAME, i+1))
                ?);
    }
    Ok(qs)
}
/**
 * Deletes the queues created for the run
 * @arg nb: the number of queues to delete (i.e. the number of followers + 1)
 * @return: Ok on success
 **/
fn unlink_queues(nb: u8) -> Result<()>{
    for i in 0..nb {
        posixmq::remove_queue(&format!("{}_{}", QNAME, i))?;
    }
    Ok(())
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
fn run_follower(id: u8, exe: &CStr, args: &[&CStr], env: &[&CStr]) -> Result<i32> {
    println!("Lauching {id}: {:?} {:?} {:?}", env, exe, args);
    match fork() {
        Ok(Fork::Parent(child)) => {
            println!("Child: {}", child);
            Ok(child)
        },
        Ok(Fork::Child) => {
            let qo = posixmq::OpenOptions::writeonly() //the follower will send messages to the
                                                       //leader on this queue
                .max_msg_len(10)
                .capacity(NB_FOLLOWER as usize)
                .create()
                .open(&format!("{}_{}", QNAME, 0))
                .expect("failed to open queue qo for a follower");
            qo.set_cloexec(false)?;

            let qi = posixmq::OpenOptions::readonly() //the follower will receive the messages of
                                                      //the leader on this queue
                .max_msg_len(10)
                .capacity(NB_FOLLOWER as usize)
                .create()    
                .open(&format!("{}_{}", QNAME, id))
                .expect("failed to open queue qi for a follower");
            qi.set_cloexec(false)?;

            execve(exe, args, env)?;
            Ok(0)
        }
        Err(_) => Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Fork failed"))),
    } 
}

fn decision_process(states: &mut [State], msg: Message) -> Result<()> {
    match msg {
        Message::Progressed(id) => {
            println!("Received Progressed from {}", id);
            //some progress happened, let's wake up every process (except id)
            //we could add context (e.g. on which file descriptor it is waiting) to guess which
            //process should be woke up
            //this would be easier if writing syscalls were also recorded
            let ids = (0..states.len()).filter(|&i| i != (id+1) as usize);
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

fn messages_handler(states: &mut [State], events: &mut BTreeMap<u64, Vec<u8>>, message: &Buffer, qs: &Vec<PosixMq>) -> Result<()> {
    match deserialize(*message) {
        Message::AddStep(id, t) => {
            println!("{:?}", events);
            println!("Received AddStep {} from {}", t, id);
            if let Some(x) = events.get_mut(&t) {
                println!("first case");
                x.push(id);
            } else {
                println!("other case");
                events.insert(t, vec![id]);
            }
            println!("{:?}", events);
        },
        Message::DelStep(id, t) => {
            println!("Received DelStep {} from {}", t, id);
            if let Some(x) = events.get_mut(&t) {
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
            let msg = serialize(Message::WakeUp(*(events.first_key_value().unwrap().0)));
            qs[id as usize].send(2, &msg.buffer)?;
            println!("response WakeUp at {id}, {:?}", msg.buffer);
        },
        Message::GetRand(id) => {
            println!("Received GetRand from {}", id);
            let msg = serialize(Message::WakeUp(RANDOM_NUMBER));
            qs[id as usize].send(2, &msg.buffer)?;
            println!("response WakeUp at {id}, {:?}", msg.buffer);
        },
        Message::Finished(id) => {
            println!("Node {} has finished", id);
            states[(id-1) as usize] = State::Finished;
            // TODO: manage this
        },
        msg => {
            decision_process(states, msg)?;
        },
    }
    Ok(())
}

fn main_loop(qs: Vec<PosixMq>) -> Result<()> {
    let mut states: [State; NB_FOLLOWER as usize] = [State::Running; NB_FOLLOWER as usize];
    let mut events: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
    let mut msg: Buffer = Buffer { buffer: [0; 10] };
    events.insert(0, vec![0]);
    loop {
        match qs[0].recv(&mut msg.buffer) {
            Ok(_) => {
                println!("States before messages_handler {:?}", states);
                messages_handler(&mut states, &mut events, &msg, &qs)?;
                println!("States after messages_handler {:?}", states);
                let mut nb_blocked = 0;
                let mut nb_finished = 0;
                for (s, q) in states.iter_mut().zip(qs[1..].iter()) {
                    match s {
                        State::Blocked => nb_blocked += 1,
                        State::ToWakeUp => {
                                let msg = serialize(Message::WakeUp(*(events.first_key_value().unwrap().0)));
                                q.send(1, &msg.buffer)?;
                                println!("response WakeUp at {:?}, {:?}", q, msg.buffer);
                                *s = State::Running;
                        },
                        State::Finished => nb_finished += 1,
                        _ => {},
                    }
                    
                }
                if nb_finished == NB_FOLLOWER {
                    break;
                }
                if nb_blocked+nb_finished == NB_FOLLOWER { //all followers are blocked or finished, we need to make a step in time
                    if let None = events.pop_first() {
                        // the simulation is finished
                        break;
                    }
                    else
                    {
                        let msg = serialize(Message::WakeUp(*(events.first_key_value().unwrap().0)));
                        for (s, q) in states.iter_mut().zip(qs[1..].iter()) {
                            if s != &State::Finished {
                                q.send(1, &msg.buffer)?;
                                println!("response WakeUp at {:?}, {:?}", q, msg.buffer);
                                *s = State::Running;
                            }
                        }
                    }
                }

                println!("States after everything {:?}", states);
            },
            Err(e) =>  {
                eprintln!("Message error: {e}");
                panic!("recv on leader queue failed");
            },
        }

        
    }
    Ok(())
}

fn main() {
    println!("Hello, world!");
    for i in 0..NB_FOLLOWER { //start the followers
        run_follower(i+1, EXE_NAMES[i as usize], &EXE_ARGS[i as usize],
            &[CString::new((format!("ID={}", i+1)).to_string().as_str()).unwrap().as_c_str(),
            CString::new((format!("LD_PRELOAD={}", LIB_NAME)).to_string().as_str()).unwrap().as_c_str()]).expect("run follower failed");
    }
    let qs = init_queues(NB_FOLLOWER).expect("init queue failed"); //open the communication queues
    /*for q in &qs[1..] {
        q.send(1, b"Born?").expect("send first message failed");
    }*/

    main_loop(qs).expect("main loop failed");

    std::thread::sleep(std::time::Duration::from_millis(500));
    unlink_queues(NB_FOLLOWER).expect("unlink failed"); //delete all communication queues
}