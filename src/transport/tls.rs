use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{channel, Sender};
use std::thread;

struct TlsTransport {
    conn: rustls::StreamOwned<rustls::ClientSession, TcpStream>,
}

impl Transport for  TlsTransport {
    fn send_packet(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let length = data.len();
        let data2 = [((length >> 8) & 0xff) as u8, (length & 0xff) as u8].iter().cloned().chain(data.iter().cloned()).collect::<Vec<_>>();
        self.conn.write_all(&data2)?;
        Ok(())
    }

    fn read_packet(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut lenbuf = [0u8; 2];
        self.conn.read_exact(&mut lenbuf)?;
        let length = (lenbuf[0] as usize) << 8 | lenbuf[1] as usize;
        let mut databuf = vec![0u8; length];
        self.conn.read_exact(&mut databuf)?;
        Ok(databuf)
    }

    fn close(&mut self) -> Result<(), Box<dyn Error>> {
        self.conn.shutdown()?;
        Ok(())
    }
}

fn new_tls_transport(host: &str, port: &str) -> Result<TlsTransport, Box<dyn Error>> {
    let remote_addr = format!("{}:{}", host, port);
    let mut config = rustls::ClientConfig::new();
    config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);

    let dns_name = webpki::DNSNameRef::try_from_ascii_str(host)?;
    let tls_session = rustls::ClientSession::new(&std::sync::Arc::new(config), dns_name);
    let tcp_stream = TcpStream::connect(&remote_addr)?;
    let mut tls_stream = rustls::StreamOwned::new(tls_session, tcp_stream);

    let (status_sender, status_receiver) = channel();
    let (data_sender, data_receiver) = channel();
    thread::spawn(move || bind_tls_stream_events(&mut tls_stream, data_sender, status_sender));

    match status_receiver.recv()? as &str {
        "open" => Ok(TlsTransport { conn: tls_stream }),
        status => Err(format!("Unexpected stream status: {}", status).into()),
    }
}
