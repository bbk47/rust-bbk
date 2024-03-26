use std::io::{Error, Read, Write};
use std::net::TcpStream;


use super::Transport;

pub struct Http2Transport {
    h2socket: TcpStream
}


unsafe impl Sync for Http2Transport {}
unsafe impl Send for Http2Transport {}

impl Transport for Http2Transport {
    fn send_packet(&self, data: &[u8]) -> Result<(), Error> {
        // let length = data.len();
        // let data2 = [((length >> 8) & 0xff) as u8, (length & 0xff) as u8].iter().cloned().chain(data.iter().cloned()).collect::<Vec<_>>();
        // self.h2socket.write_all(&data2)?;
        Ok(())
    }

    fn read_packet(&self) -> Result<Vec<u8>, Error> {
        let mut lenbuf = [0u8; 2];
        self.h2socket.read_exact(&mut lenbuf)?;
        let length = (lenbuf[0] as usize) << 8 | lenbuf[1] as usize;
        let mut databuf = vec![0u8; length];
        self.h2socket.read_exact(&mut databuf)?;
        Ok(databuf)
    }

    fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}