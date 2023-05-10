use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

struct TcpTransport {
    conn: TcpStream,
}

impl Transport for  TcpTransport {
    fn send_packet(&mut self, data: &[u8]) -> Result<(), String> {
        self.conn.write_all(data).map_err(|e| e.to_string())
    }

    fn read_packet(&mut self) -> Result<Vec<u8>, String> {
        let mut lenbuf = [0u8; 2];
        self.conn.read_exact(&mut lenbuf).map_err(|e| e.to_string())?;

        let length = (lenbuf[0] as usize) << 8 | lenbuf[1] as usize;
        let mut databuf = vec![0u8; length];
        self.conn.read_exact(&mut databuf).map_err(|e| e.to_string())?;

        Ok(databuf)
    }

    fn close(&mut self) -> Result<(), String> {
        self.conn.shutdown(std::net::Shutdown::Both).map_err(|e| e.to_string())
    }
}

fn new_tcp_transport<A: ToSocketAddrs>(addr: A) -> Result<TcpTransport, String> {
    let conn = TcpStream::connect_timeout(&addr, Duration::from_secs(10)).map_err(|e| e.to_string())?;

    Ok(TcpTransport { conn })
}
