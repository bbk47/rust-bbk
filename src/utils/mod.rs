use std::{
    io,
    net::TcpStream,
    sync::Arc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log::{error, info};

use crate::stub::VirtualStream;

pub mod encrypt;
pub mod socks5;
pub mod uuid;

pub fn get_timestamp() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).expect("Clock may have gone backwards");
    let millis = duration.as_millis() as i64;
    format!("{}", millis)
}

pub fn forward( tcpstream: TcpStream, vstream: Arc<VirtualStream>) {
    let mut browser_socket1 = tcpstream.try_clone().unwrap();
    let mut browser_socket2 = tcpstream.try_clone().unwrap();
    let mut v_stream1 = vstream.try_clone().unwrap();
    let mut v_stream2 = vstream.try_clone().unwrap();

    thread::spawn(move || {
        let ret = io::copy(&mut browser_socket1, &mut v_stream1);
        match ret {
            Ok(_) => {}
            Err(err) => {
                error!("forward err:{:?}",err.to_string());
            }
        }
    });
    thread::spawn(move || {
        let ret = io::copy(&mut v_stream2, &mut browser_socket2);
        match ret {
            Ok(_) => {
                info!("copy stream to browser complete.");
                v_stream2.close();
                browser_socket2.shutdown(std::net::Shutdown::Write);
            }
            Err(err) => {
                error!("forward err:{:?}",err.to_string());
            }
        }
    });
}
