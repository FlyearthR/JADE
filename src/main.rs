const NB_FOLLOWER: usize = 3;
use posixmq::PosixMq;
use std::io::Result;

fn init_queues(nb: usize) -> Result<Vec<PosixMq>>  {
    let mut qs = Vec::new();
    for i in 0..nb {
        let t = PosixMq::create(&format!("/nts_mq_{}", i))?;
        t.set_cloexec(false)?;
        qs.push(t);
    }
    Ok(qs)
}

fn main() {
    println!("Hello, world!");
    let mut ret: u32;
    let mut failed: bool = false;
    let qs = init_queues(NB_FOLLOWER).unwrap();

}
