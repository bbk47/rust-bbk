use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::sync::Mutex;

enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

struct Logger {
    level: LogLevel,
    writer: Mutex<Box<dyn Write + Send>>,
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Logger {
            level,
            writer: Mutex::new(Box::new(io::stdout())),
        }
    }

    pub fn set_writer(&self, writer: Box<dyn Write + Send>) {
        *self.writer.lock().unwrap() = writer;
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level >= self.level {
            let mut writer = self.writer.lock().unwrap();
            writeln!(writer.as_mut(), "{}: {}", level_str(level), message).unwrap();
        }
    }

    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }
}

fn level_str(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    }
}

// fn main() {
//     let logger = Logger::new(LogLevel::Info);

//     // 记录 INFO 级别的日志
//     logger.info("Hello, world!");

//     // 记录 DEBUG 级别的日志（但由于日志级别设为 INFO，因此不会输出）
//     logger.debug("This is a debug message.");

//     // 将日志写入文件
//     logger.set_writer(Box::new(BufWriter::new(File::create("log.txt").unwrap())));

//     // 记录 WARN 级别的日志，将会输出到文件中
//     logger.warn("Something went wrong.");
// }
