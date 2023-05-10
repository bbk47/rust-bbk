use std::error::Error;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

struct AbcTlsServer {
    listener: TcpListener,
}

impl AbcTlsServer {
    fn new(host: &str, port: u16, ssl_crt_path: &str, ssl_key_path: &str) -> Result<Self, Box<dyn Error>> {
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

        Ok(AbcTlsServer { listener })
    }
}

trait FrameServer {
    fn listen_conn(&self, handler: impl Fn(TunnelConn) + Send + Sync + 'static);
    fn listen_http_conn(&self, handler: impl Fn(http::Request<()>) -> http::Response<Vec<u8>> + Send + Sync + 'static);
    fn get_addr(&self) -> String;
}

impl FrameServer for AbcTlsServer {
    fn listen_conn(&self, handler: impl Fn(TunnelConn) + Send + Sync + 'static) {
        let mut accepted_conns = Arc::new(Mutex::new(Vec::new()));
        let conn_handler = move |mut stream: TcpStream| {
            let tcp_socket = stream.try_clone().unwrap();

            let wrap_conn = TunnelConn {
                tuntype: "tls".to_owned(),
                wsocket: Arc::new(websocket::sync::Client::new(&stream).unwrap()),
                tcp_socket,
                h2_socket: h2conn::Conn::new(stream.try_clone().unwrap()).unwrap(),
            };

            let conn_handler = handler.clone();
            let accepted_conns = accepted_conns.clone();

            std::thread::spawn(move || {
                accepted_conns.lock().unwrap().push(tcp_socket.try_clone().unwrap());
                conn_handler(wrap_conn);
            });
        };

        for conn in self.listener.incoming() {
            if let Ok(stream) = conn {
                let conn_handler = conn_handler.clone();

                std::thread::spawn(move || conn_handler(stream));
            }
        }
    }

    fn listen_http_conn(&self, _handler: impl Fn(http::Request<()>) -> http::Response<Vec<u8>> + Send + Sync + 'static) {
        // nothing to do
    }

    fn get_addr(&self) -> String {
        format!("tls://{}", self.listener.local_addr().unwrap())
    }
}
