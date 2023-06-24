use std::{error::Error, io::Write, net::TcpStream};

pub trait Transport {
    fn send_packet(&mut self, data: &[u8]) -> Result<(), std::io::Error>;
    fn read_packet(&mut self) -> Result<Vec<u8>, std::io::Error>;
    fn close(&self) -> Result<(), std::io::Error>;
}

pub fn send_stream_socket(socket: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    let length = data.len();
    let mut data2 = Vec::with_capacity(length + 2);
    data2.push((length >> 8) as u8);
    data2.push((length & 0xff) as u8);
    data2.extend_from_slice(data);
    socket.write_all(&data2)
}

// pub fn send_ws_socket(wss: &mut websocket::sender, data: &[u8]) -> Result<()> {
//     wss.write_message(websocket::OwnedMessage::Binary(data.to_vec()))
// }
