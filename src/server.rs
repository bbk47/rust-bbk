use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

use crate::BbkSerOption;
pub struct BbkServer {
    opts: BbkSerOption,
}

impl BbkServer {
    pub fn new(opts: BbkSerOption) -> Self {
        println!("server new====");
        BbkServer { opts: opts }
    }

    pub fn bootstrap(&self) {
        let addr = format!("{}:{}", self.opts.listen_addr, self.opts.listen_port);
        let listener = TcpListener::bind(&addr).unwrap();

        let hellobytes = "hello client i server!".as_bytes();
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            thread::spawn(move || loop {
                let mut buffer = [0; 1024];
                let size = stream.read(&mut buffer).unwrap();
                // println!("size {}", size);
                println!("Get Msg: {}", String::from_utf8_lossy(&buffer[..size]));
                // stream.write(hellobytes).unwrap()
            });
            println!("Connection established!");
        }
    }
}
