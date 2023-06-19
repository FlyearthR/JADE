const NB_FOLLOWER: usize = 3;
const QNAME = "/nts_mq";
use posixmq::PosixMq;
use std::io::Result;
use fork::{fork, Fork};
use nix::unistd::execve;
use std::ffi::CStr;
use std::ffi::CString;

fn init_queues(nb: usize) -> Result<Vec<PosixMq>> {
    let mut qs = Vec::new();
    for i in 0..nb {
        let t = PosixMq::create(&format!("{}_{}", QNAME, i))?;
        t.set_cloexec(false)?;
        qs.push(t);
    }
    Ok(qs)
}

fn unlink_queues(nb: usize) -> Result<()>{
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
        Err(e) => Err(e),
    } 
}

fn main() {
    println!("Hello, world!");
    let qs = init_queues(NB_FOLLOWER).unwrap();
    

    unlink_queues(NB_FOLLOWER).unwrap();
}
