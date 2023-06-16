use std::{io::{self, Read, Write}, net::TcpStream};

pub struct ProxySocket {
    addr_buf: Vec<u8>,
    conn: TcpStream,
}

impl ProxySocket {
    // 关联方法new：构造Rectangle的实例对象
    pub fn new(addr_buf: Vec<u8>, conn: TcpStream) -> ProxySocket {
        ProxySocket { addr_buf, conn }
    }
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.conn.read(buf)
    }

    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.conn.write(buf)
    }

    pub fn close(&mut self) -> io::Result<()> {
        self.conn.shutdown(std::net::Shutdown::Both)
    }

    pub fn get_addr(&self) -> &[u8] {
        &self.addr_buf[..]
    }
}
