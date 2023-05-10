use std::error::Error;
use std::sync::mpsc::{channel, Sender};
use std::thread;

use websocket::{ClientBuilder, OwnedMessage};

struct WebsocketTransport {
    conn: websocket::WebSocketStream<std::net::TcpStream>,
}

impl Transport for  WebsocketTransport {
    fn send_packet(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let message = OwnedMessage::Binary(data.to_vec());
        self.conn.send_message(&message)?;
        Ok(())
    }

    fn read_packet(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let message = self.conn.read_message()?;
        match message {
            OwnedMessage::Binary(data) => Ok(data),
            _ => Err("Unexpected message type".into()),
        }
    }

    fn close(&mut self) -> Result<(), Box<dyn Error>> {
        self.conn.shutdown()?;

        // Waiting for graceful connection closure.
        while self.conn.read_message()?.is_close() {}
        Ok(())
    }
}

fn new_websocket_transport(host: &str, port: &str, path: &str, secure: bool) -> Result<WebsocketTransport, Box<dyn Error>> {
    let ws_url = if secure { format!("wss://{}{}", host, path) } else { format!("ws://{}:{}{}", host, port, path) };
    println!("transport wsurl: {}", ws_url);
    let mut client = ClientBuilder::new(&ws_url)?.connect_insecure()?;
    let stream = client.get_mut();
    let (status_sender, status_receiver) = channel();
    let (data_sender, data_receiver) = channel();
    thread::spawn(move || bind_ws_stream_events(stream, data_sender, status_sender));

    match status_receiver.recv()? as &str {
        "open" => Ok(WebsocketTransport { conn: client }),
        status => Err(format!("Unexpected stream status: {}", status).into()),
    }
}
