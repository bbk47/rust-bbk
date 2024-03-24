use std::net::TcpStream;

pub struct ProxySocket {
    addr_buf: Vec<u8>,
    pub conn: TcpStream,
}

impl ProxySocket {
    // 关联方法new：构造Rectangle的实例对象
    pub fn new(addr_buf: Vec<u8>, conn: TcpStream) -> ProxySocket {
        ProxySocket { addr_buf, conn }
    }
    pub fn get_addr(&self) -> &[u8] {
        &self.addr_buf[..]
    }
}
