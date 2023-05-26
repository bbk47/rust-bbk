use std::io::{self, Read, Write};
use std::net::TcpStream;

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

fn main() -> io::Result<()> {
    let left = TcpStream::connect("127.0.0.1:4000")?;
    let right = TcpStream::connect("127.0.0.1:5000")?;

    relay(&left, &right)?;

    Ok(())
}
