use std::error::Error;
use std::net::{TcpListener, TcpStream};

use super::base::FrameServer;
use super::TunnelConn;

// TCP 监听器
pub struct AbcHttp2Server {
    listener: TcpListener,
}

impl AbcHttp2Server {
    pub fn new(ln: TcpListener) -> AbcHttp2Server {
        AbcHttp2Server { listener: ln }
    }
}

impl FrameServer for AbcHttp2Server {

    fn get_addr(&self) -> String {
        format!("tcp://{}", self.listener.local_addr().unwrap())
    }
    
    fn accept(&mut self)->Result<TunnelConn,Box<dyn Error>> {
        match self.listener.accept() {
            Ok((stream, _addr)) => {
                let socket = stream;
                let tuntype: String = String::from("h2");
                let tuncon = TunnelConn {
                    tuntype,
                    websocket: None,
                    tcp_socket: Some(socket),
                    tls_socket:None
                };
                Ok(tuncon)
            }
            Err(e) => Err(Box::new(e))
        }
    }
}
