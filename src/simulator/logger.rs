use std::fs::File;
use std::io::Write;
use std::time::SystemTime;

use futures::executor::block_on;
use futures::io::BufWriter;



pub struct Logger{
    log_file: File,
    trace: bool, // path through the code
    debug: bool, // call + args
    warn: bool, 
    error: bool, 
    info: bool, //other
}

// impl Drop for Logger{
//     fn drop(&mut self){
        
//     }
// }

impl Logger{
    pub fn new(file_name:&str) -> Self{
        let n = "../logs/".to_owned()+file_name;
        let log_file = File::create(n).expect("creation failed");
        Self{
            log_file: log_file,
            trace: true,
            debug: true,
            warn: true,
            error: true,
            info: true
        }
    }

    pub fn log(&self, level:&str, message:&str){
        let timestamp = SystemTime::now();
        match level{
            "trace" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "{:?} [TRACE] {}",timestamp, message){
                    println!("{:?} [ERROR] could not write to log file",timestamp);
                }
            }
            "debug" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "{:?} [DEBUG] {}",timestamp, message){
                    println!("{:?} [ERROR] could not write to log file",timestamp);
                }
            }
            "warn" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "{:?} [WARN] {}",timestamp, message){
                    println!("{:?} [ERROR] could not write to log file",timestamp);
                }
            }
            "error" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "{:?} [ERROR] {}",timestamp, message){
                    println!("{:?} [ERROR] could not write to log file",timestamp);
                }
            }
            "info" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "{:?} [INFO] {}",timestamp, message){
                    println!("{:?} [ERROR] could not write to log file",timestamp);
                }
            }
            _ => {
                println!("{:?} [ERROR] log type {} doesn't exist",timestamp,level);
            }
        }
    }
}