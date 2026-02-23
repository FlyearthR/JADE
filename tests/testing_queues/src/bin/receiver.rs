use libc::*;
use std::ffi::CString;
use std::ptr;
use std::mem;

fn main() {
    let name = CString::new("/example_mq").unwrap();

    unsafe {
        let mut attr: mq_attr = mem::zeroed();
        attr.mq_flags = 0;
        attr.mq_maxmsg = 10;
        attr.mq_msgsize = 128;
        attr.mq_curmsgs = 0;

        let mqd = mq_open(
            name.as_ptr(),
            O_CREAT | O_RDONLY,
            0o600,
            &attr,
        );

        if mqd == -1 {
            panic!("mq_open failed");
        }

        let mut buffer = vec![0u8; 128];
        let mut prio: u32 = 0;

        let n = mq_receive(
            mqd,
            buffer.as_mut_ptr() as *mut i8,
            buffer.len(),
            &mut prio,
        );

        if n < 0 {
            panic!("mq_receive failed");
        }

        let msg = String::from_utf8_lossy(&buffer[..n as usize]);
        println!("Received: {}", msg);

        mq_close(mqd);
        mq_unlink(name.as_ptr());
    }
}
