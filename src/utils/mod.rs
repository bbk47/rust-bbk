use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub mod encrypt;
pub mod logger;
pub mod socks5;
pub mod uuid;

pub fn get_timestamp() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).expect("Clock may have gone backwards");
    let millis = duration.as_millis() as i64;
    format!("{}", millis)
}
