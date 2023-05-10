mod http2;
mod websocket;

use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
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
            let st = stream.unwrap();
            let stream_clone = st.try_clone().unwrap();
            let mut reader = BufReader::new(st);
            let mut writer = BufWriter::new(stream_clone);
            thread::spawn(move || loop {
                let mut buffer = [0; 1024];
                let size = reader.read(&mut buffer).unwrap();
                println!("Get Msg from client: {}", String::from_utf8_lossy(&buffer[..size]));
                writer.write(hellobytes).unwrap();
                writer.flush().unwrap();
            });
        }
    }
}
