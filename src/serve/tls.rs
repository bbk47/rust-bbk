use std::any::Any;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::net::{TcpListener};
use std::path::Path;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

use std::result::Result;

use super::base::TunnelConn;
use super::base::FrameServer;

const NTHREADS: usize = 8;

struct AbcTlsServer {
    listener: TcpListener,
}

impl FrameServer for AbcTlsServer {
    fn listen_conn(&self, handler: impl Fn(&TunnelConn) + Send + Sync + 'static) -> Result<(), Box<dyn Error>>{
        let pool = ThreadPool::new(NTHREADS);
        let shared_handler = Arc::new(Mutex::new(handler));
        for stream in self.listener.incoming() {
            let shared_handler = shared_handler.clone();
            let shared_stream = Arc::new(stream?);        
            pool.execute(move || {
                let tunnel_conn = TunnelConn::new("tls".to_owned(), shared_stream.to_owned());
                let handler = shared_handler.lock().unwrap();
                handler(&tunnel_conn).unwrap();
            });
        }
        Ok(())
    }

    fn get_addr(&self) -> String {
        format!("tls://{}", self.listener.local_addr().unwrap())
    }
}


pub fn new_abc_tls_server(host: &str, port: u16, ssl_crt_path: &str, ssl_key_path: &str) -> Result<Box<dyn FrameServer>, Box<dyn Error>> {
    let addr = format!("{}:{}", host, port);
    let ssl_crt_file = File::open(Path::new(ssl_crt_path))?;
    let mut ssl_crt_reader = BufReader::new(ssl_crt_file);
    let mut ssl_crt_contents = Vec::new();
    ssl_crt_reader.read_to_end(&mut ssl_crt_contents)?;

    let ssl_key_file = File::open(Path::new(ssl_key_path))?;
    let mut ssl_key_reader = BufReader::new(ssl_key_file);
    let mut ssl_key_contents = Vec::new();
    ssl_key_reader.read_to_end(&mut ssl_key_contents)?;

    let cert = openssl::x509::X509::from_pem(&ssl_crt_contents)?;
    let key = openssl::pkey::PKey::private_key_from_pem(&ssl_key_contents)?;

    let tls_acceptor = openssl::ssl::SslAcceptor::mozilla_modern(openssl::ssl::SslMethod::tls())?;
    tls_acceptor.set_private_key(&key)?;
    tls_acceptor.set_certificate(&cert)?;
    tls_acceptor.check_private_key()?;

    let listener = TcpListener::bind(addr)?;
    let listener = tls_acceptor.accept_listener(listener)?;
    let server = AbcTlsServer { listener };
    Ok(Box::new(server))
}
