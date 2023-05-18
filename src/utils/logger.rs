use std::cmp::Ordering;
use std::io::{self, Write};
use std::sync::Mutex;

#[derive(PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}



impl PartialOrd for LogLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (&LogLevel::Error, &LogLevel::Error) => Some(Ordering::Equal),
            (&LogLevel::Error, _) => Some(Ordering::Greater),
            (_, &LogLevel::Error) => Some(Ordering::Less),
            (&LogLevel::Warn, &LogLevel::Warn) => Some(Ordering::Equal),
            (&LogLevel::Warn, _) => Some(Ordering::Greater),
            (_, &LogLevel::Warn) => Some(Ordering::Less),
            (&LogLevel::Info, &LogLevel::Info) => Some(Ordering::Equal),
            (&LogLevel::Info, _) => Some(Ordering::Greater),
            (_, &LogLevel::Info) => Some(Ordering::Less),
            (&LogLevel::Debug, &LogLevel::Debug) => Some(Ordering::Equal),
        }
    }
}

impl Ord for LogLevel {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (&LogLevel::Error, &LogLevel::Error) 
            | (&LogLevel::Warn, &LogLevel::Warn)
            | (&LogLevel::Info, &LogLevel::Info)
            | (&LogLevel::Debug, &LogLevel::Debug) => Ordering::Equal,
            (&LogLevel::Error, _) => Ordering::Greater,
            (_, &LogLevel::Error) => Ordering::Less,
            (&LogLevel::Warn, &LogLevel::Info) 
            | (&LogLevel::Warn, &LogLevel::Debug) 
            | (&LogLevel::Info, &LogLevel::Debug) => Ordering::Greater,
            (_, _) => Ordering::Less,
        }
    }
}


pub struct Logger {
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
