use std::{
    io,
    net::TcpStream,
    sync::Arc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log::{error, info};
use regex::bytes;

use crate::stub::VirtualStream;

pub mod emiter;
pub mod encrypt;
pub mod socks5;
pub mod uuid;

pub fn get_timestamp() -> i64 {
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Failed to get current time");
    let timestamp = current_time.as_secs() as i64 * 1000 + current_time.subsec_nanos() as i64 / 1_000_000;
    timestamp
}

pub fn get_timestamp_bytes() -> Vec<u8> {
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Failed to get current time");
    let timestamp = current_time.as_secs() as i64 * 1000 + current_time.subsec_nanos() as i64 / 1_000_000;

    timestamp.to_string().as_bytes().to_vec()
}

pub fn forward(tcpstream: TcpStream, vstream: Arc<VirtualStream>) {
    let mut tcpstream1 = tcpstream.try_clone().unwrap();
    let mut tcpstream2 = tcpstream.try_clone().unwrap();
    let mut v_stream1 = vstream.try_clone().unwrap();
    let mut v_stream2 = vstream.try_clone().unwrap();

    thread::spawn(move || {
        let ret = io::copy(&mut tcpstream1, &mut v_stream1);
        match ret {
            Ok(_) => {
                info!("forward: copy stream to vstream complete1.");
                v_stream1.close();
            }
            Err(err) => {
                error!("forward err:{:?}", err.to_string());
                v_stream1.close();
            }
        }
    });
    let ret = io::copy(&mut v_stream2, &mut tcpstream2);
    match ret {
        Ok(_) => {
            info!("forward: copy vstream to stream complete2.");
            tcpstream2.shutdown(std::net::Shutdown::Write);
        }
        Err(err) => {
            error!("forward err:{:?}", err.to_string());
            tcpstream2.shutdown(std::net::Shutdown::Both);
        }
    }
    info!("forward=====complete....");
}
