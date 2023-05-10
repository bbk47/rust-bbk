use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};

struct Stream {
    cid: String,
    addr: Vec<u8>,
    ts: Arc<TunnelStub>,
    rp: io::Result<io::PipeReader>,
    wp: io::Result<io::PipeWriter>,
}

impl Stream {
    fn new(cid: String, addr: Vec<u8>, ts: Arc<TunnelStub>) -> Self {
        let (rp, wp) = io::pipe().unwrap();
        Stream {
            cid,
            addr,
            ts,
            rp: Ok(rp),
            wp: Ok(wp),
        }
    }

    fn produce(&mut self, data: &[u8]) -> io::Result<()> {
        // println!("produce wp====:{:x?}", data);
        self.wp.as_mut().unwrap().write_all(data)
    }
}

impl Read for Stream {
    fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
        self.rp.as_mut().unwrap().read(data)
    }
}

impl Write for Stream {
    fn write(&mut self, p: &[u8]) -> io::Result<usize> {
        // println!("write stream[{}] data:{:x?}", self.cid, p);
        let mut buf2 = vec![0; p.len()];
        buf2.copy_from_slice(p); // Copy data to avoid overwriting buffer due to delayed consumption by the target writer
        self.ts.send_data_frame(self.cid.clone(), buf2)?;
        Ok(p.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        // log::debug!("closing ch");
        let _ = self.rp.as_mut().unwrap().close();
        let _ = self.wp.as_mut().unwrap().close();
    }
}

fn relay(mut left: impl Read + Write, mut right: impl Read + Write) -> io::Result<()> {
    let (mut left_clone, mut right_clone) = (left.try_clone()?, right.try_clone()?);

    // Thread 1: read from right and write to left
    let thread1 = std::thread::spawn(move || io::copy(&mut right, &mut left_clone));

    // Thread 2: read from left and write to right
    let thread2 = std::thread::spawn(move || io::copy(&mut left, &mut right_clone));

    thread1.join().unwrap()?;
    thread2.join().unwrap()?;

    Ok(())
}
