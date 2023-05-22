use std::error::Error;
use std::io::Result;
use std::sync::{Arc, Mutex};
use std::{io, thread};

use std::net::{TcpListener, TcpStream};

use super::{FrameServer, TunnelConn};

const NTHREADS: usize = 8;

// TCP 监听器
pub struct AbcTcpServer {
    listener: TcpListener,
}

impl AbcTcpServer {
    pub fn new(host: &str, port: u16) -> io::Result<AbcTcpServer> {
        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(addr)?;
        Ok(AbcTcpServer {
            listener,
        })
    }
}

impl Iterator for AbcTcpServer {
    type Item = Result<TunnelConn>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.listener.accept() {
            Ok((stream, _addr)) =>{
                let socket = stream;
                let tuntype:String = String::from("tcp");
                 let tuncon = TunnelConn{tuntype,tcp_socket:socket};
                 Some(Ok(tuncon))
            },
            Err(e) => Some(Err(e)),
        }
    }
}

impl FrameServer for AbcTcpServer {

    fn get_addr(&self) -> String {
        format!("tcp://{}", self.listener.local_addr().unwrap())
    }
}
