use std::error::Error;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Sender};
use std::thread;

use http::{Request, Uri};
use hyper::{Body, Client};
use hyper_tls::HttpsConnector;

struct Http2Transport {
    h2socket: hyper::client::Http2Stream,
}

impl Transport for Http2Transport {
    fn send_packet(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let length = data.len();
        let data2 = [((length >> 8) & 0xff) as u8, (length & 0xff) as u8].iter().cloned().chain(data.iter().cloned()).collect::<Vec<_>>();
        self.h2socket.write_all(&data2)?;
        Ok(())
    }

    fn read_packet(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut lenbuf = [0u8; 2];
        self.h2socket.read_exact(&mut lenbuf)?;
        let length = (lenbuf[0] as usize) << 8 | lenbuf[1] as usize;
        let mut databuf = vec![0u8; length];
        self.h2socket.read_exact(&mut databuf)?;
        Ok(databuf)
    }

    fn close(&mut self) -> Result<(), Box<dyn Error>> {
        self.h2socket.close()?;
        Ok(())
    }
}

fn new_http2_transport(host: &str, port: &str, path: &str) -> Result<Http2Transport, Box<dyn Error>> {
    let uri = format!("https://{}:{}{}", host, port, path).parse::<Uri>().unwrap();
    let https = HttpsConnector::new().unwrap();
    let client = Client::builder().build::<_, Body>(https);
    let req = Request::method(http::Method::POST, uri).body(Body::empty()).unwrap();

    let (status_sender, status_receiver) = channel();
    let (data_sender, data_receiver) = channel();
    let mut stream = client.request(req)?.into_parts().1;
    thread::spawn(move || bind_h2c_stream_events(stream, data_sender, status_sender));

    match status_receiver.recv()? as &str {
        "open" => Ok(Http2Transport { h2socket: data_receiver.into() }),
        status => Err(format!("Unexpected stream status: {}", status).into()),
    }
}
