use posixmq::PosixMq;
//use std::io::ErrorKind;
use std::io::Result;
use fork::{fork, Fork};
use nix::unistd::execve;
//use nix::sched::setns;
use std::ffi::CStr;
use std::ffi::CString;
use std::collections::BTreeMap;
//use core::time::Duration;
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
fn leader_init_queues(nb: u8) -> Result<Vec<PosixMq>> {
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
 * Open one queue on which the follower will send message to the leader and
 * one queue on which the follower will receive messages from the server
 * @arg id: the id of the follower, strictly positive
 * @return: on success return a pair of posix queues (follower_receiving_queue, follower_sending_queue)
 */
fn follower_init_queues(id: u8) -> Result<(PosixMq, PosixMq)> {
    let qo = posixmq::OpenOptions::writeonly() //the follower will send messages to the
        .max_msg_len(10)                           //leader on this queue
        .capacity(NB_FOLLOWER as usize)
        .create()
        .open(&format!("{}_{}", QNAME, 0))
        .expect("failed to open queue qo for a follower");
    qo.set_cloexec(false)?;

    let qi = posixmq::OpenOptions::readonly() //the follower will receive the messages of
        .max_msg_len(10)                          //the leader on this queue
        .capacity(NB_FOLLOWER as usize)
        .create()    
        .open(&format!("{}_{}", QNAME, id))
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
            let _ = follower_init_queues(id);
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

fn main_loop(qs: Vec<PosixMq>) -> Result<u8> {
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
                        State::Finished => {
                            nb_finished += 1;
                            println!("nb_finished incremented");
                        },
                        _ => {},
                    }
                    
                }
                println!("number finished: {nb_finished}");
                if nb_finished == NB_FOLLOWER {
                    return Ok(0);
                }
                if nb_blocked+nb_finished == NB_FOLLOWER { //all followers are blocked or finished, we need to make a step in time
                    let _ = events.pop_first();
                    if let None = events.first_key_value() {
                        // the simulation is finished
                        println!("Simulation finished by all process beeing blocked with no more events");
                        return Ok(1);
                    }
                    let msg = serialize(Message::WakeUp(*(events.first_key_value().unwrap().0)));
                    for (s, q) in states.iter_mut().zip(qs[1..].iter()) {
                        if s != &State::Finished {
                            q.send(1, &msg.buffer)?;
                            println!("response WakeUp at {:?}, {:?}", q, msg.buffer);
                            *s = State::Running;
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
}

fn main() {
    println!("Hello, world!");
    for i in 0..NB_FOLLOWER { //start the followers
        run_follower(i+1, EXE_NAMES[i as usize], &EXE_ARGS[i as usize],
            &[CString::new((format!("ID={}", i+1)).to_string().as_str()).unwrap().as_c_str(),
            CString::new((format!("LD_PRELOAD={}", LIB_NAME)).to_string().as_str()).unwrap().as_c_str()]).expect("run follower failed");
    }
    let qs = leader_init_queues(NB_FOLLOWER).expect("init queue failed"); //open the communication queues
    /*for q in &qs[1..] {
        q.send(1, b"Born?").expect("send first message failed");
    }*/

    main_loop(qs).expect("main loop failed");

    std::thread::sleep(std::time::Duration::from_millis(500));
    unlink_queues(NB_FOLLOWER).expect("unlink failed"); //delete all communication queues
}

#[cfg(test)]
mod simple_network {
    use network_time_simulator::Buffer;

    //use network_time_simulator::{deserialize, serialize};
    use crate::{main_loop, Message::*};
    use crate::{serialize, deserialize};

    use crate::{follower_init_queues, leader_init_queues, unlink_queues};
    use serial_test::serial;
    use std::thread;

    #[test]
    #[serial]
    fn test_init_queues() {
        let qs = leader_init_queues(3).expect("creating leader queues");
        let mut buf = vec![0; 100];
        assert_eq!(qs.len(), 4);
        for i in 1..4 {
            let qio = follower_init_queues(i).expect("creating follower queues");
            qs[i as usize].send(10, b"Born?").expect("send first message failed");
            assert_eq!(qio.0.recv(&mut buf).unwrap(), (10, "Born?".len()));
            qio.1.send(10, b"Yes!").expect("send first message failed");
            assert_eq!(qs[0].recv(&mut buf).unwrap(), (10, "Yes!".len()));
        }
        unlink_queues(3).expect("error during unlinking");
    }

    #[test]
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
        let qio = follower_init_queues(1).expect("follower queues creating");
        let mut msg: Buffer = Buffer { buffer: [0; 10] };
        let msgs = [Progressed(42), Stuck(42), AddStep(42, 43), DelStep(42, 42), GetTime(42), GetRand(42), Finished(42)];

        for m in msgs {
            let check = m.clone();
            qio.1.send(10, &serialize(m).buffer).expect("send follower message failed");
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
        let qf1 = follower_init_queues(1).expect("follower queues creating");
        let qf2 = follower_init_queues(2).expect("follower queues creating");
        let t = thread::spawn(|| {main_loop(qs)});
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
            let mut msg: Buffer = Buffer { buffer: [0; 10] };
            qf1.0.recv(&mut msg.buffer).unwrap();
            assert_eq!(deserialize(msg), WakeUp(i));
            qf2.0.recv(&mut msg.buffer).unwrap();
            assert_eq!(deserialize(msg), WakeUp(i));
        }
        qf1.1.send(1, &serialize(Finished(1)).buffer).expect("send finished failed");
        qf2.1.send(1, &serialize(Finished(2)).buffer).expect("send finished failed");
        if let Ok(nb) = t.join().unwrap() {
            let _ = unlink_queues(2);
            assert_eq!(nb, return_value);
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
}