use std::any::Any;
use std::error::Error;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use super::base::{FrameServer, TunnelConn};
use std::result::Result;

const NTHREADS: usize = 8;

pub struct AbcWssServer {
    listener: TcpListener,
}

impl FrameServer for AbcWssServer {
    fn listen_conn(&self) -> Result<(), Box<dyn Error>> {
        // let shared_handler = Arc::new(Mutex::new(handler));
        // thread::spawn( move {
        //     loop {
        //         let (stream, addr) = self.listener.accept().await?;
        //         let shared_handler = shared_handler.clone();
        //         let tunnel_conn = TunnelConn::new("tcp".to_owned(), stream);
        //         thread::spawn( move {
        //             let handler = shared_handler.lock().unwrap();
        //             handler(&tunnel_conn);
        //         });
        //     }
        // });
        Ok(())
    }

    fn get_addr(&self) -> String {
        format!("tcp://{}", self.listener.local_addr().unwrap())
    }
}

pub fn new_abc_wss_server(host: &str, port: u16, path: &str) -> Result<Box<dyn FrameServer>, Box<dyn Error>> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(addr)?;
    let server = AbcWssServer { listener };
    Ok(Box::new(server) as Box<dyn FrameServer>)
}
