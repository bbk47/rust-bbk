use std::any::Any;
use std::error::Error;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

use super::base::FrameServer;
use super::base::TunnelConn;
use std::result::Result;

const NTHREADS: usize = 8;

pub struct AbcTcpServer {
    listener: TcpListener,
}

impl FrameServer for AbcTcpServer {
    fn listen_conn(&self, handler: impl Fn(&TunnelConn) + Send + Sync + 'static) -> Result<(), Box<dyn Error>> {
        // let pool = ThreadPool::new(NTHREADS);
        // let shared_handler = Arc::new(Mutex::new(handler));
        // for stream in self.listener.incoming() {
        //     let shared_handler = shared_handler.clone();
        //     let shared_stream = Arc::new(stream?);
        //     pool.execute(move || {
        //         let tunnel_conn = TunnelConn::new("tcp".to_owned(), (&*shared_stream).to_owned());
        //         let handler = shared_handler.lock().unwrap();
        //         handler(&tunnel_conn);
        //     });
        // }
        Ok(())
    }
    fn get_addr(&self) -> String {
        format!("tcp://{}", self.listener.local_addr().unwrap())
    }
}

pub fn new_abc_tcp_server(host: &str, port: u16) -> Result<Box<AbcTcpServer>, Box<dyn Error>> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(addr)?;
    let server = AbcTcpServer { listener };
    Ok(Box::new(server))
}
