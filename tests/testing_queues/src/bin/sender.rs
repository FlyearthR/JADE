use libc::*;
use std::ffi::CString;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let name = CString::new("/example_mq").unwrap();

    // Give receiver time to create the queue
    sleep(Duration::from_secs(1));

    unsafe {
        let mqd = mq_open(
            name.as_ptr(),
            O_WRONLY,
        );

        if mqd == -1 {
            panic!("mq_open failed");
        }

        let msg = "Hello from sender!";
        let c_msg = CString::new(msg).unwrap();

        if mq_send(
            mqd,
            c_msg.as_ptr(),
            msg.len(),
            1,
        ) != 0
        {
            panic!("mq_send failed");
        }

        println!("Message sent.");

        mq_close(mqd);
    }
}
