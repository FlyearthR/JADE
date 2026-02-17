use std::fs::File;
use std::io::Write;

/// Simple logger for recording simulation events at various levels.
pub struct Logger {
    /// The file where logs are written.
    log_file: File,
    /// Enable trace level logs.
    trace: bool, // path through the code
    /// Enable debug level logs.
    debug: bool, // call + args
    /// Enable warn level logs.
    warn: bool,
    /// Enable error level logs.
    error: bool,
    /// Enable info level logs.
    info: bool, //other
    /// Enable message level logs.
    message: bool,
}

impl std::fmt::Debug for Logger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Logger : trace:{} debug:{} warn:{} error:{} info:{} message:{}",
            self.trace, self.debug, self.warn, self.error, self.info, self.message
        )
    }
}

impl Logger {
    pub fn new(
        file_name: &str,
        trace: bool,
        debug: bool,
        warn: bool,
        error: bool,
        info: bool,
        message: bool,
    ) -> Self {
        let log_file = File::create(file_name).expect("creation failed");
        Self {
            log_file: log_file,
            trace: trace,
            debug: debug,
            warn: warn,
            error: error,
            info: info,
            message: message,
        }
    }

    /// Logs a message at the specified level if enabled.
    pub fn log(&self, level: &str, message: &str) {
        match level {
            "trace" => {
                if let Err(e) = writeln!(&mut &self.log_file, "[TRACE] {}", message) {
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            "debug" => {
                if let Err(e) = writeln!(&mut &self.log_file, "[DEBUG] {}", message) {
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            "warn" => {
                if let Err(e) = writeln!(&mut &self.log_file, "[WARN] {}", message) {
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            "error" => {
                if let Err(e) = writeln!(&mut &self.log_file, "[ERROR] {}", message) {
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            "info" => {
                if let Err(e) = writeln!(&mut &self.log_file, "[INFO] {}", message) {
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            "message" => {
                if let Err(e) = writeln!(&mut &self.log_file, "[MESSAGE] {}", message) {
                    println!("[ERROR] could not write to log file : {}", e);
                }
            }
            _ => {
                println!("[ERROR] log type {} doesn't exist", level);
            }
        }
    }
}
