use std::any::Any;
use std::error::Error;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use super::base::{FrameServer, TunnelConn};
use std::result::Result;

const NTHREADS: usize = 8;

pub struct AbcHttp2Server {
    listener: TcpListener,
}

impl FrameServer for AbcHttp2Server {
    fn listen_conn(&self) -> Result<(), Box<dyn Error>> {
        // let shared_handler = Arc::new(Mutex::new(handler));
        // tokio::spawn(async move {
        //     loop {
        //         let (stream, addr) = self.listener.accept().await?;
        //         let shared_handler = shared_handler.clone();
        //         let tunnel_conn = TunnelConn::new("tcp".to_owned(), stream);
        //         tokio::spawn(async move {
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

pub fn new_abc_http2_server(host: &str, port: u16, path:&str, ssl_crt_path: &str, ssl_key_path: &str) -> Result<Box<dyn FrameServer>, Box<dyn Error>> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(addr)?;
    let server = AbcHttp2Server { listener };
    Ok(Box::new(server) as Box<dyn FrameServer>)
}
