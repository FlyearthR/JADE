use std::fs::File;
use std::io::Write;


#[derive(Debug)]
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
    pub fn new(file_name:&str,trace:bool,debug:bool,warn:bool,error:bool,info:bool) -> Self{
        let n = "../logs/".to_owned()+file_name;
        let log_file = File::create(n).expect("creation failed");
        Self{
            log_file: log_file,
            trace: trace,
            debug: debug,
            warn: warn,
            error: error,
            info: info
        }
    }

    pub fn log(&self, level:&str, message:&str){
        match level{
            "trace" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "[TRACE] {}", message){
                    println!("[ERROR] could not write to log file");
                }
            }
            "debug" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "[DEBUG] {}", message){
                    println!("[ERROR] could not write to log file");
                }
            }
            "warn" =>{
                    if let Err(e)=writeln!(&mut &self.log_file, "[WARN] {}", message){
                    println!("[ERROR] could not write to log file");
                }
            }
            "error" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "[ERROR] {}", message){
                    println!("[ERROR] could not write to log file");
                }
            }
            "info" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "[INFO] {}", message){
                    println!("[ERROR] could not write to log file");
                }
            }
            _ => {
                println!("[ERROR] log type {} doesn't exist",level);
            }
        }
    }
}