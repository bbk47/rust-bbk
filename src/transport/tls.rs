use std::cell::UnsafeCell;
use std::io::{Error, Read, Write};
use std::net::TcpStream;

use native_tls::TlsStream;

use super::Transport;

pub struct TlsTransport {
    pub conn: UnsafeCell<TlsStream<TcpStream>>,
}

unsafe impl Sync for TlsTransport {}
unsafe impl Send for TlsTransport {}

impl Transport for TlsTransport {
    fn send_packet(&self, data: &[u8]) -> Result<(), Error> {
        let length = data.len();
        let mut data2 = Vec::with_capacity(length + 2);
        data2.push((length >> 8) as u8);
        data2.push((length & 0xff) as u8);
        data2.extend_from_slice(data);
        unsafe {
            let data_ptr = self.conn.get();
            let data_ref = &mut *data_ptr;
            data_ref.write_all(&data2)?;
            Ok(())
        }
    }

    fn read_packet(&self) -> Result<Vec<u8>, Error> {
        let mut lenbuf = [0u8; 2];
        unsafe {
            let data_ptr = self.conn.get();
            let data_ref = &mut *data_ptr;
            data_ref.read_exact(&mut lenbuf)?;
            let length = (lenbuf[0] as usize) << 8 | lenbuf[1] as usize;
            // println!("len:{}", length);
            let mut databuf = vec![0u8; length];
            data_ref.read_exact(&mut databuf)?;
            Ok(databuf)
        }
    }

    fn close(&self) -> Result<(), Error> {
        println!("close tls transport");
        unsafe {
            let data_ptr = self.conn.get();
            let data_ref = &mut *data_ptr;
            data_ref.shutdown()?;
            Ok(())
        }
    }
}
