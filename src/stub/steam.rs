use std::borrow::BorrowMut;
use std::error::Error;
use std::io::{self, Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};

pub struct CopyStream {
    pub cid: String,
    pub addr: Vec<u8>,
    wp: Sender<Vec<u8>>,
    rp: Receiver<Vec<u8>>,
    wp2: Sender<Vec<u8>>,
    rp2: Receiver<Vec<u8>>,
}

unsafe impl Sync for CopyStream {}

unsafe impl Send for CopyStream {}



impl CopyStream {
    pub fn new(cid: String, addr: Vec<u8>) -> Self {
        let (mut wp, mut rp): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();
        let (mut wp2, mut rp2): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();
        CopyStream { cid, addr, rp, wp, rp2, wp2 }
    }

    pub fn produce(&self, buf: &[u8]) {
        let _ = self.wp.send(buf.to_vec());
    }

    // pub fn accept(&self) -> Result<Vec<u8>, dyn Error> {
    //     match self.rp2.try_recv() {
    //         Ok(data) => Some(Ok(data)),
    //         Err(e) => Some(Err(e)),
    //     }
    // }
}

impl Read for CopyStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // let mut reader = self.rp.borrow_mut();
        // reader.read(buf)
        match self.rp.try_recv() {
            Ok(data) => {
                let len = data.len().min(buf.len());
                buf[0..len].copy_from_slice(&data[0..len]);
                Ok(len)
            }
            Err(_) => Err(io::ErrorKind::WouldBlock.into()),
        }
    }
}

impl Write for CopyStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut writer = self.wp2.borrow_mut();
        let ret = writer.send(buf.to_vec()).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // no cache
        Ok(())
    }
}

// pub fn relay(mut left: impl Read + Write, mut right: impl Read + Write) -> io::Result<()> {
//     let (mut left_clone, mut right_clone) = (left.try_clone()?, right.try_clone()?);

//     // Thread 1: read from right and write to left
//     let thread1 = std::thread::spawn(move || io::copy(&mut right, &mut left_clone));

//     // Thread 2: read from left and write to right
//     let thread2 = std::thread::spawn(move || io::copy(&mut left, &mut right_clone));

//     thread1.join().unwrap()?;
//     thread2.join().unwrap()?;

//     Ok(())
// }
