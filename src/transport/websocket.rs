use std::io::{Error, ErrorKind, Read, Write};
use std::net::TcpStream;

use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};
use std::cell::UnsafeCell;

use super::Transport;

pub struct WsTransport {
    pub conn: UnsafeCell<WebSocket<TcpStream>>,
}

unsafe impl Sync for WsTransport {}
unsafe impl Send for WsTransport {}

impl Transport for WsTransport {
    fn send_packet(&self, data: &[u8]) -> Result<(), Error> {
        unsafe {
            let data_ptr = self.conn.get();
            let data_ref = &mut *data_ptr;
            match data_ref.send(Message::Binary(data.to_vec())) {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::new(ErrorKind::WriteZero, e)),
            }
        }
    }

    fn read_packet(&self) -> Result<Vec<u8>, Error> {
        unsafe {
            let data_ptr = self.conn.get();
            let data_ref = &mut *data_ptr;
            match data_ref.read(){
                Ok(msg)=>Ok(msg.into_data()),
                Err(e)=>Err(Error::new(ErrorKind::Other, format!("Read websocket err!{}",e.to_string())))
            }
        }
    }

    fn close(&self) -> Result<(), Error> {
        unsafe {
            let data_ptr = self.conn.get();
            let data_ref = &mut *data_ptr;
            data_ref.close(None).unwrap();
            Ok(())
        }
    }
}

pub struct WssTransport {
    pub conn: UnsafeCell<WebSocket<MaybeTlsStream<TcpStream>>>,
}

unsafe impl Sync for WssTransport {}
unsafe impl Send for WssTransport {}

impl Transport for WssTransport {
    fn send_packet(&self, data: &[u8]) -> Result<(), Error> {
        unsafe {
            let data_ptr = self.conn.get();
            let data_ref = &mut *data_ptr;
            match data_ref.send(Message::Binary(data.to_vec())) {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::new(ErrorKind::WriteZero, e)),
            }
        }
    }

    fn read_packet(&self) -> Result<Vec<u8>, Error> {
        unsafe {
            let data_ptr = self.conn.get();
            let data_ref = &mut *data_ptr;
            match data_ref.read(){
                Ok(msg)=>Ok(msg.into_data()),
                Err(e)=>Err(Error::new(ErrorKind::Other, format!("Read websocket err!{}",e.to_string())))
            }
        }
    }

    fn close(&self) -> Result<(), Error> {
        unsafe {
            let data_ptr = self.conn.get();
            let data_ref = &mut *data_ptr;
            data_ref.close(None).unwrap();
            Ok(())
        }
    }
}
