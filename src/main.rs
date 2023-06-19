use posixmq::PosixMq;
use std::io::Result;
use fork::{fork, Fork};
use nix::unistd::execve;
use std::ffi::CStr;
use std::ffi::CString;

const NB_FOLLOWER: u8 = 3;
const QNAME: &str = "/nts_mq";
const EXE_NAME: &str = "./callee";

fn init_queues(nb: u8) -> Result<Vec<PosixMq>> {
    let mut qs = Vec::new();
    for i in 0..nb {
        qs.push(PosixMq::create(&format!("{}_{}", QNAME, i))?);
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
            let t = PosixMq::create(&format!("{}_{}", QNAME, id))?;
            t.set_cloexec(false)?;
            execve(&CString::new(exe.as_str()).unwrap().as_c_str(), args, env);
            Ok(0)
        }
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, String::from("Fork failed"))),
    } 
}

fn main() {
    println!("Hello, world!");
    for i in 0..NB_FOLLOWER {
        run_follower(i, String::from(EXE_NAME), &[], &[]).unwrap();
    }

    let qs = init_queues(NB_FOLLOWER).unwrap();

    unlink_queues(NB_FOLLOWER).unwrap();
}
