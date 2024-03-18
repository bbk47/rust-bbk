use log::{Level, LevelFilter, Log, Metadata, Record};
use std::io::Write;

pub struct CustomLogger;

impl Log for CustomLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut buf = std::io::stdout();

            // 根据日志级别设置颜色
            match record.level() {
                Level::Error => {
                    let _ = write!(buf, "\x1b[31m"); // 红色
                }
                Level::Warn => {
                    let _ = write!(buf, "\x1b[33m"); // 黄色
                }
                Level::Info => {
                    let _ = write!(buf, "\x1b[32m"); // 绿色
                }
                Level::Debug | Level::Trace => {
                    let _ = write!(buf, "\x1b[34m"); // 蓝色
                }
            }

            let _ = writeln!(buf, "{} - {} - {}", record.level(), record.target(), record.args());

            let _ = write!(buf, "\x1b[0m"); // 重置颜色
        }
    }

    fn flush(&self) {}
}