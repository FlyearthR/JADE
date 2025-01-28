use std::fs::File;
use std::io::Write;



pub struct Logger{
    log_file: File,
    trace: bool, // path through the code
    debug: bool, // call + args
    warn: bool, 
    error: bool, 
    info: bool, //other
    message: bool,
}

impl std::fmt::Debug for Logger{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Logger : trace:{} debug:{} warn:{} error:{} info:{} message:{}", self.trace, self.debug, self.warn, self.error, self.info, self.message)
    }
}

impl Logger{
    pub fn new(file_name:&str,trace:bool,debug:bool,warn:bool,error:bool,info:bool, message:bool) -> Self{
        let n = "../logs/".to_owned()+file_name;
        let log_file = File::create(n).expect("creation failed");
        Self{
            log_file: log_file,
            trace: trace,
            debug: debug,
            warn: warn,
            error: error,
            info: info,
            message: message,
        }
    }

    pub fn log(&self, level:&str, message:&str){
        match level{
            "trace" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "[TRACE] {}", message){
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            "debug" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "[DEBUG] {}", message){
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            "warn" =>{
                    if let Err(e)=writeln!(&mut &self.log_file, "[WARN] {}", message){
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            "error" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "[ERROR] {}", message){
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            "info" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "[INFO] {}", message){
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            "message" =>{
                if let Err(e)=writeln!(&mut &self.log_file, "[MESSAGE] {}", message){
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            _ => {
                println!("[ERROR] log type {} doesn't exist",level);
            }
        }
    }
}