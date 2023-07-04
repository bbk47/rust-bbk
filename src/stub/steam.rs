use std::io;
use std::io::{Read, Write};
use std::sync::Arc;

use std::sync::mpsc::{sync_channel, Receiver, RecvError, Sender, SyncSender};

use crate::protocol::{self, Frame};
use crate::utils::socks5::AddrInfo;

pub struct VirtualStream {
    pub cid: String,
    pub addstr: String,
    pub addr: Vec<u8>,
    tx1: SyncSender<Vec<u8>>,
    rp1: Arc<Receiver<Vec<u8>>>,
    sender: SyncSender<Frame>,
    current: Vec<u8>,
    current_pos: usize,
}

unsafe impl Sync for VirtualStream {}
unsafe impl Send for VirtualStream {}

impl VirtualStream {
    pub fn new(cid: String, addr: Vec<u8>, sender: SyncSender<Frame>) -> Self {
        let (tx1, rp1): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = sync_channel(0);

        let addrinfo = AddrInfo::from_buffer(&addr).unwrap();
        let addstr = format!("{}:{}", &addrinfo.host, &addrinfo.port);

        VirtualStream {
            cid,
            addr,
            addstr,
            rp1: Arc::new(rp1),
            tx1,
            sender,
            current: Vec::new(),
            current_pos: 0,
        }
    }

    pub fn produce(&self, buf: &[u8]) {
        // provide data
        let _ = self.tx1.send(buf.to_vec());
    }

    pub fn close(&self) {
        self.produce("".as_bytes())
    }

    pub fn shutdown(&self) -> std::io::Result<()> {
        // close stream
        self.produce("".as_bytes());
        let frame = Frame::new(self.cid.to_owned(), protocol::FIN_FRAME, vec![0x1, 0x2]);
        self.sender.send(frame).unwrap();
        println!("shuwdown....");
        Ok(())
    }
    pub fn try_clone(&self) -> Option<Self> {
        let cid = self.cid.clone();
        let addr = self.addr.clone();
        let tx1 = self.tx1.clone();
        let rp1 = self.rp1.clone();
        let sender = self.sender.clone();
        let addstr = self.addstr.clone();
        let cloned = VirtualStream {
            cid,
            addr,
            addstr,
            rp1,
            tx1,
            sender,
            current: Vec::new(),
            current_pos: 0,
        };
        Some(cloned)
    }
}

impl Read for VirtualStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }

        loop {
            let remaining = self.current.len() - self.current_pos;
            if remaining > 0 {
                let to_fill = std::cmp::min(remaining, buf.len());
                buf[..to_fill].copy_from_slice(&self.current[self.current_pos..(self.current_pos + to_fill)]);
                self.current_pos += to_fill;
                return Ok(to_fill);
            }
            // block thread
            match self.rp1.recv() {
                Ok(b) => {
                    self.current = b;
                    self.current_pos = 0;
                }
                Err(_) => return Ok(0),
            };
        }
    }
}

impl Write for VirtualStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let frame = Frame::new(self.cid.to_owned(), protocol::STREAM_FRAME, buf.to_vec());
        self.sender.send(frame).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // no cache
        Ok(())
    }
}
