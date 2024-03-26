use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read};
use std::net::TcpListener;
use std::sync::Arc;

use native_tls::{Identity, TlsAcceptor};

use super::base::{FrameServer, TunnelConn};

// TCP 监听器
pub struct AbsTlsServer {
    listener: TcpListener,
    acceptor: Arc<TlsAcceptor>,
}

impl AbsTlsServer {
    pub fn new(ln: TcpListener, ssl_crt: &str, ssl_key: &str) -> AbsTlsServer {
        let identity = Identity::from_pkcs8(ssl_crt.as_bytes(),ssl_key.as_bytes()).unwrap();
        let acceptor = TlsAcceptor::new(identity).unwrap();
        let acceptor = Arc::new(acceptor);

        AbsTlsServer { listener: ln, acceptor }
    }
}

impl FrameServer for AbsTlsServer {
    fn get_addr(&self) -> String {
        format!("tcp://{}", self.listener.local_addr().unwrap())
    }

    fn accept(&mut self) -> Result<TunnelConn, Box<dyn Error>> {
        match self.listener.accept() {
            Ok((stream, _addr)) => {
                let acceptor = self.acceptor.clone();
                let stream = acceptor.accept(stream)?;

                let tuntype: String = String::from("tls");
                let tuncon = TunnelConn {
                    tuntype,
                    websocket: None,
                    tcp_socket: None,
                    tls_socket: Some(stream),
                };
                Ok(tuncon)
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

// pub fn new_abc_tls_server(host: &str, port: u16, ssl_crt_path: &str, ssl_key_path: &str) -> Result<AbsTlsServer,> {
//     let addr = format!("{}:{}", host, port);
//     let ssl_crt_file = File::open(Path::new(ssl_crt_path))?;
//     let mut ssl_crt_reader = BufReader::new(ssl_crt_file);
//     let mut ssl_crt_contents = Vec::new();
//     ssl_crt_reader.read_to_end(&mut ssl_crt_contents)?;

//     let ssl_key_file = File::open(Path::new(ssl_key_path))?;
//     let mut ssl_key_reader = BufReader::new(ssl_key_file);
//     let mut ssl_key_contents = Vec::new();
//     ssl_key_reader.read_to_end(&mut ssl_key_contents)?;

//     let cert = openssl::x509::X509::from_pem(&ssl_crt_contents)?;
//     let key = openssl::pkey::PKey::private_key_from_pem(&ssl_key_contents)?;

//     let tls_acceptor = openssl::ssl::SslAcceptor::mozilla_modern(openssl::ssl::SslMethod::tls())?;
//     tls_acceptor.set_private_key(&key)?;
//     tls_acceptor.set_certificate(&cert)?;
//     tls_acceptor.check_private_key()?;

//     let listener = TcpListener::bind(addr)?;
//     let listener = tls_acceptor.accept_listener(listener)?;
//     let server = AbsTlsServer { listener };
//     Ok(server)
// }
