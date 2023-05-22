use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::rc::Rc;

pub struct CopyStream {
    pub cid: String,
    addr: Vec<u8>,
    reader: Rc<RefCell<dyn Read>>,
    writer: Rc<RefCell<dyn Write>>,
}

impl CopyStream {
    fn new(cid: String, addr: Vec<u8>, reader: Rc<RefCell<dyn Read>>, writer: Rc<RefCell<dyn Write>>) -> Self {
        CopyStream { cid, addr, reader, writer }
    }
}

impl Read for CopyStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut reader = self.reader.borrow_mut();
        reader.read(buf)
    }
}

impl Write for CopyStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut writer = self.writer.borrow_mut();
        writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut writer = self.writer.borrow_mut();
        writer.flush()
    }
}

pub fn relay(mut left: impl Read + Write, mut right: impl Read + Write) -> io::Result<()> {
    let (mut left_clone, mut right_clone) = (left.try_clone()?, right.try_clone()?);

    // Thread 1: read from right and write to left
    let thread1 = std::thread::spawn(move || io::copy(&mut right, &mut left_clone));

    // Thread 2: read from left and write to right
    let thread2 = std::thread::spawn(move || io::copy(&mut left, &mut right_clone));

    thread1.join().unwrap()?;
    thread2.join().unwrap()?;

    Ok(())
}
