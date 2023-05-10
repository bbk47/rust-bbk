use std::net::TcpStream;

pub trait Transport {
    fn send_packet(&mut self, data: &[u8]) -> std::io::Result<()>;
    fn read_packet(&mut self) -> std::io::Result<Vec<u8>>;
    fn close(&mut self) -> std::io::Result<()>;
}

pub fn wrap_tunnel(tunnel: &server::TunnelConn) -> Box<dyn Transport> {
    match tunnel.tuntype.as_str() {
        "ws" => Box::new(WebsocketTransport { conn: tunnel.wsocket.clone() }),
        "h2" => Box::new(Http2Transport { h2socket: tunnel.h2socket.clone() }),
        "tcp" => Box::new(TcpTransport {
            conn: tunnel.tcpsocket.try_clone().unwrap(),
        }),
        _ => Box::new(TlsTransport {
            conn: tunnel.tcpsocket.try_clone().unwrap(),
        }),
    }
}

pub fn send_stream_socket(socket: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    let length = data.len();
    let mut data2 = Vec::with_capacity(length + 2);
    data2.push((length >> 8) as u8);
    data2.push((length & 0xff) as u8);
    data2.extend_from_slice(data);
    socket.write_all(&data2)
}

pub fn send_ws_socket(wss: &mut websocket::WebSocketWriter, data: &[u8]) -> std::io::Result<()> {
    wss.write_message(websocket::OwnedMessage::Binary(data.to_vec()))
}
