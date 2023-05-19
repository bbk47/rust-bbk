use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;

use futures::{StreamExt};

use tokio::io::{AsyncReadExt, AsyncWriteExt, self};
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::mpsc;

use super::base::Incoming;
use super::FrameServer;

const NTHREADS: usize = 8;

// TCP 监听器
pub struct AbcTcpServer {
    listener: TcpListener,
    incoming_sender: Arc<Mutex<mpsc::UnboundedSender<io::Result<TcpStream>>>>,
}

impl AbcTcpServer {
    pub async fn bind(addr: &str) -> io::Result<AbcTcpServer> {
        let listener = TcpListener::bind(addr).await?;
        let (connection_sender, connection_receiver) = mpsc::unbounded_channel();
        let incoming_sender = Arc::new(Mutex::new(connection_sender));
        Ok(AbcTcpServer {
            listener,
            incoming_sender,
        })
    }
}

impl FrameServer for AbcTcpServer {
    fn incoming(&self) -> Incoming {
        let incoming_sender = self.incoming_sender.clone();
        let incoming_receiver = incoming_sender.lock().unwrap().clone();
        Incoming::new(incoming_sender, incoming_receiver)
    }

    fn get_addr(&self) -> String {
        format!("tcp://{}", self.listener.local_addr().unwrap())
    }
}

pub async fn new_abc_wss_server(host: &str, port: u16, path: &str) -> io::Result<Box<dyn FrameServer>> {
    let addr = format!("{}:{}", host, port);
    let stream = AbcTcpServer::bind(&addr).await?;
    let incoming_sender = stream.incoming_sender.clone();

    for _ in 0..NTHREADS {
        let incoming_sender = incoming_sender.clone();
        let mut incoming = stream.incoming();
        thread::spawn(move || {
            while let Some(stream) = incoming.next() {
                match stream {
                    Ok(mut stream) => {
                        let sender = incoming_sender.lock().unwrap().clone();
                        tokio::spawn(async move {
                            let mut buf = [0u8; 1024];
                            let n = stream.read(&mut buf).await.unwrap();
                            println!("Received {} bytes: {:?}", n, &buf[..n]);
                            stream.write_all(&buf[..n]).await.unwrap();
                            println!("Sent {} bytes");
                            let _ = sender.send(Ok(stream));
                        });
                    },
                    Err(e) => {
                        eprintln!("Accept error: {}", e);
                    }
                }
            }
        });
    }

    Ok(Box::new(stream) as Box<dyn FrameServer>)
}