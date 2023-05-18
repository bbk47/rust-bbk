use std::io;

pub trait ProxySocket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn close(&mut self) -> io::Result<()>;
    fn get_addr(&self) -> &[u8];
}