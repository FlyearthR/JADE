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
}

#[derive(Copy, Clone)]
enum State {
    Running,
    Blocked,
}

fn init_queues(nb: u8) -> Result<Vec<PosixMq>> {
    let mut qs = Vec::new();
    qs.push(posixmq::OpenOptions::readonly()
            .max_msg_len(24)
            .capacity(NB_FOLLOWER as usize)
            .create()
            .open(&format!("{}_{}", QNAME, 0))
            ?);

    for i in 0..nb {
        qs.push(posixmq::OpenOptions::writeonly()
                .max_msg_len(24)
                .capacity(NB_FOLLOWER as usize)
                .create()
                .open(&format!("{}_{}", QNAME, i+1))
                ?);
    }
    Ok(qs)
}

fn unlink_queues(nb: u8) -> Result<()>{
    for i in 0..nb {
        posixmq::remove_queue(&format!("{}_{}", QNAME, i))?;
    }
    Ok(())
}

fn run_follower(id: u8, exe: String, args: &[&CStr], env: &[&CStr]) -> Result<i32> {
    match fork() {
        Ok(Fork::Parent(child)) => Ok(child),
        Ok(Fork::Child) => {
            let qo = posixmq::OpenOptions::writeonly()
                .max_msg_len(24)
                .capacity(NB_FOLLOWER as usize)
                .create()
                .open(&format!("{}_{}", QNAME, 0))
                .unwrap();
            qo.set_cloexec(false)?;

            let qi = posixmq::OpenOptions::readonly()
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

fn main_loop(qs: Vec<PosixMq>) -> Result<()> {
    let mut states: [State; NB_FOLLOWER as usize] = [State::Running; NB_FOLLOWER as usize];
    let mut events: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
    loop {
        let mut msg: [u8; 24] = [0; 24];
        qs[0].recv(&mut msg)?;
        match serde_cbor::from_slice(&msg).unwrap() {
            Message::Progressed(ID) => {
                states[ID as usize] = State::Blocked;
            },
            Message::Stuck(ID) => {
                states[ID as usize] = State::Blocked;
            },
            Message::AddStep(ID, t) => {
                if let Some(x) = events.get_mut(&t) {
                    x.push(ID);
                } else {
                    events.insert(t, Vec::from([ID]));
                }
            },
            Message::DelStep(ID, t) => {
                if let Some(x) = events.get_mut(&t) {
                    for elem in x.iter_mut() {
                        if elem == &ID {
                            *elem = 0;
                            break;
                        }
                    }
                }
            },
            Message::GetTime(ID) => break,
        }
    }
    Ok(())
}

fn main() {
    println!("Hello, world!");
        for i in 0..NB_FOLLOWER {
        run_follower(i+1, String::from(EXE_NAME), &[], &[]).unwrap();
    }
    let qs = init_queues(NB_FOLLOWER).unwrap();
    for q in &qs[1..] {
        q.send(1, b"Born?").unwrap();
    }

    main_loop(qs);

    std::thread::sleep(std::time::Duration::from_millis(500));
    unlink_queues(NB_FOLLOWER).unwrap();
}
