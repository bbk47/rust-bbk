use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::BbkCliOption;

pub struct BbkClient {
    opts: BbkCliOption,
}

impl BbkClient {
    pub fn new(opts: BbkCliOption) -> Self {
        println!("client new====");
        BbkClient { opts: opts }
    }

    pub fn bootstrap(self) {
        let tunopts = match self.opts.tunnel_opts {
            Some(tp) => tp,
            None => panic!("missing tunnelOpts config"),
        };
        let addr = format!("{}:{}", tunopts.host, tunopts.port);
        let stream = match TcpStream::connect(&addr) {
            Ok(stream) => stream,
            Err(e) => {
                panic!("Failed to connect: {}", e);
            }
        };

        println!("Successfully connected to server in port {}", &addr);
        let mstream = Arc::new(Mutex::new(stream));
        let stream2 = Arc::clone(&mstream);

        thread::spawn(move || loop {
            let hellobytes = "hello server i client!".as_bytes();
            println!("lock spawn....");
            {
                let ret = stream2.lock().unwrap().write(&hellobytes);
                if let Err(e) = ret {
                    println!("write err:{}", e);
                    return;
                }
                if let Ok(s) = ret {
                    println!("write size:{}", s);
                }
            }
            thread::sleep(Duration::from_millis(10));
        });
        loop {
            println!("read====");
            thread::sleep(Duration::from_millis(1));
            let mut buffer = [0; 1024];
            {
                println!("====lock====");
                mstream.lock().unwrap().read(&mut buffer).unwrap();
                println!("Get Msg: {}", String::from_utf8_lossy(&buffer[..]));
            }
        }
    }
}
