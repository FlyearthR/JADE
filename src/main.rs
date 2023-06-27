use posixmq::PosixMq;
use std::io::Result;
use fork::{fork, Fork};
use nix::unistd::execve;
use std::ffi::CStr;
use std::ffi::CString;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

const NB_FOLLOWER: u8 = 3;
const QNAME: &str = "/nts_mq";
const EXE_NAME: &str = "./callee";

#[derive(Serialize, Deserialize)]
enum Message {
    Progressed(u8),
    Stuck(u8),
    AddStep(u8, u64),
    DelStep(u8, u64),
    GetTime(u8),
    WakeUp,
}

#[derive(Copy, Clone)]
enum State {
    Running,
    Waiting,
    Blocked,
}
/**
 * Initialize one queue on which the leader will wait for the messages from the followers and a
 * queue by follower
 * @arg nb: the number of follower
 * @return: on succes return a vector of posix queue, the first one beeing the leader's queue
 **/

fn init_queues(nb: u8) -> Result<Vec<PosixMq>> {
    let mut qs = Vec::new();
    qs.push(posixmq::OpenOptions::readonly() //the leader will receive messages on this queue
            .max_msg_len(24)
            .capacity(NB_FOLLOWER as usize)
            .create()
            .open(&format!("{}_{}", QNAME, 0))
            ?);

    for i in 0..nb {
        qs.push(posixmq::OpenOptions::writeonly() //the leader will send messages on those queues
                .max_msg_len(24)
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
fn run_follower(id: u8, exe: String, args: &[&CStr], env: &[&CStr]) -> Result<i32> {
    match fork() {
        Ok(Fork::Parent(child)) => Ok(child),
        Ok(Fork::Child) => {
            let qo = posixmq::OpenOptions::writeonly() //the follower will send messages to the
                                                       //leader on this queue
                .max_msg_len(24)
                .capacity(NB_FOLLOWER as usize)
                .create()
                .open(&format!("{}_{}", QNAME, 0))
                .unwrap();
            qo.set_cloexec(false)?;

            let qi = posixmq::OpenOptions::readonly() //the follower will receive the messages of
                                                      //the leader on this queue
                .max_msg_len(24)
                .capacity(NB_FOLLOWER as usize)
                .create()    
                .open(&format!("{}_{}", QNAME, id))
                .unwrap();
            qi.set_cloexec(false)?;

            execve(&CString::new(exe.as_str()).unwrap().as_c_str(), args, env)?;
            Ok(0)
        }
        Err(_) => Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Fork failed"))),
    } 
}

fn decision_process(states: &[State], runnings: u8, id:u8, newState: State, qs: &Vec<PosixMq>) -> Result<()> {
    match newState {
        State::Waiting => {
            //some progress happened, let's wake up any process (except id)
            //we could add context (e.g. on which file descriptor it is waiting) to guess which
            //process should be woke up
            //this would be easier if writing syscalls were also recorded
            let idToWakeUp = 1; //should try random
            let sts = &states[idToWakeUp..];
            let qss = &qs[idToWakeUp+1..];
            for (s,q) in sts.iter().zip(qss[idToWakeUp+1..].iter()) {
                match s {
                    State::Running => continue,
                    _ => {
                        if let Ok(msg) = serde_cbor::to_vec(&Message::WakeUp) {
                            let m = &msg[..];
                            q.send(1, m);
                        }
                        else {
                            return Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Message not properly serialized")))
                        }
                        break;
                    },
                }
            }
            Ok(())
        },
        State::Blocked => {
            //did not progress
            //woke up the wrong process, try again
            Ok(())
        },
        _ => {
            //this should not happen
            Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Unexpected message")))
        },
    }
}

fn main_loop(qs: Vec<PosixMq>) -> Result<()> {
    let mut runnings = NB_FOLLOWER;
    let mut states: [State; NB_FOLLOWER as usize] = [State::Running; NB_FOLLOWER as usize];
    let mut events: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
    loop {
        let mut msg: [u8; 24] = [0; 24];
        qs[0].recv(&mut msg)?;
        match serde_cbor::from_slice(&msg).unwrap() {
            Message::Progressed(id) => {
                states[id as usize] = State::Waiting;
                runnings -= 1;
                decision_process(&states, runnings, id, State::Waiting, &qs);
            },
            Message::Stuck(id) => {
                states[id as usize] = State::Blocked;
                runnings -= 1;
                decision_process(&states, runnings, id, State::Blocked, &qs);
            },
            Message::AddStep(id, t) => {
                if let Some(x) = events.get_mut(&t) {
                    x.push(id);
                } else {
                    events.insert(t, Vec::from([id]));
                }
            },
            Message::DelStep(id, t) => {
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
                //return head key of the BTreeMap
                break;
            },
            _ => {
            //this should not happen
            return Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Unexpected message")))
        },
        }
    }
    Ok(())
}

fn main() {
    println!("Hello, world!");
        for i in 0..NB_FOLLOWER { //start the followers
        run_follower(i+1, String::from(EXE_NAME), &[], &[]).unwrap();
    }
    let qs = init_queues(NB_FOLLOWER).unwrap(); //open the communication queues
    for q in &qs[1..] {
        q.send(1, b"Born?").unwrap();
    }

    main_loop(qs);

    std::thread::sleep(std::time::Duration::from_millis(500));
    unlink_queues(NB_FOLLOWER).unwrap(); //delete all communication queues
}
