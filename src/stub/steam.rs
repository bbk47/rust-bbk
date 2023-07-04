use std::io;
use std::io::{Read, Write};
use std::sync::Arc;

use std::sync::mpsc::{channel, Receiver, RecvError, Sender};

use crate::protocol::{Frame, self};


pub struct VirtualStream {
    pub cid: String,
    pub addr: Vec<u8>,
    tx1: Sender<Vec<u8>>,
    rp1: Arc<Receiver<Vec<u8>>>,
    sender:Sender<Frame>,
}

unsafe impl Sync for VirtualStream {}

unsafe impl Send for VirtualStream {}

impl VirtualStream {
    pub fn new(cid: String, addr: Vec<u8>, sender: Sender<Frame>) -> Self {
        let (tx1, rp1): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();

        VirtualStream {
            cid,
            addr,
            rp1: Arc::new(rp1),
            tx1,
            sender
        }
    }

    pub fn produce(&self, buf: &[u8]) {
        // provide data
        let _ = self.tx1.send(buf.to_vec());
    }

    pub fn shutdown(&mut self) -> std::io::Result<()> {
        // close stream
        println!("shuwdown....");
        Ok(())
    }
    pub fn try_clone(&self) -> Option<Self> {
        let cid = self.cid.clone();
        let addr = self.addr.clone();
        let tx1 = self.tx1.clone();
        let rp1 = self.rp1.clone();
        let sender = self.sender.clone();
        let cloned = VirtualStream { cid, addr, rp1, tx1,sender };
        Some(cloned)
    }
}

impl Read for VirtualStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.rp1.recv() {
            Ok(data) => {
                let len = data.len().min(buf.len());
                buf[0..len].copy_from_slice(&data[0..len]);
                let string_result = std::str::from_utf8(&buf[..len]).unwrap();
                println!("read ok:{:?},{}", string_result, len);
                // println!("read ok:{:?}",&buf);
                Ok(len)
            }
            Err(_) => {
                println!("read error");
                Err(io::ErrorKind::WouldBlock.into())
            }
        }
    }
}

impl Write for VirtualStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        println!("write to virtual stream...start.");
        let frame = Frame::new(self.cid.to_owned(), protocol::STREAM_FRAME, buf.to_vec());
        self.sender.send(frame).unwrap();
        println!("write to virtual stream...complete.");
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // no cache
        Ok(())
    }
}