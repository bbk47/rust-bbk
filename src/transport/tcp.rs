use std::io::{Read, Write, Error};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use super::base::Transport;
#[derive(Debug)]
pub struct TcpTransport {
    pub conn: TcpStream,
}

impl Transport for TcpTransport {
    fn send_packet(&mut self, data: &[u8]) -> Result<(), Error> {
        self.conn.write_all(data)?;
        Ok(())
    }

    fn read_packet(&mut self) -> Result<Vec<u8>, Error> {
        let mut lenbuf = [0u8; 2];
        self.conn.read_exact(&mut lenbuf)?;

        let length = (lenbuf[0] as usize) << 8 | lenbuf[1] as usize;
        let mut databuf = vec![0u8; length];
        self.conn.read_exact(&mut databuf)?;

        Ok(databuf)
    }

    fn close(&mut self) -> Result<(), Error> {
        self.conn.shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }
}

fn new_tcp_transport(host: &str, port: u16) -> Result<TcpTransport, Error> {
    let socket_addr = (host, port).to_socket_addrs()?.next().unwrap();
    let conn = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10))?;
    Ok(TcpTransport { conn })
}