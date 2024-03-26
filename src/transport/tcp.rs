use std::io::{Error, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

use super::base::Transport;

pub struct TcpTransport {
    pub conn: TcpStream,
}

unsafe impl Sync for TcpTransport {}
unsafe impl Send for TcpTransport {}

impl Transport for TcpTransport {
    fn send_packet(&self, data: &[u8]) -> Result<(), Error> {
        let length = data.len();
        let mut data2 = Vec::with_capacity(length + 2);
        data2.push((length >> 8) as u8);
        data2.push((length & 0xff) as u8);
        data2.extend_from_slice(data);
        (&self.conn).write_all(&data2)?;
        Ok(())
    }

    fn read_packet(&self) -> Result<Vec<u8>, Error> {
        let mut lenbuf = [0u8; 2];
        (&self.conn).read_exact(&mut lenbuf)?;

        let length = (lenbuf[0] as usize) << 8 | lenbuf[1] as usize;
        // println!("len:{}", length);
        let mut databuf = vec![0u8; length];
        (&self.conn).read_exact(&mut databuf)?;

        Ok(databuf)
    }

    fn close(&self) -> Result<(), Error> {
        println!("close tcp transport");
        self.conn.shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }
}

