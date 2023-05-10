use std::error::Error;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

struct AbcTcpServer {
    listener: TcpListener,
}

impl AbcTcpServer {
    fn new(host: &str, port: u16) -> Result<Self, Box<dyn Error>> {
        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(addr)?;

        Ok(AbcTcpServer { listener })
    }
}

impl FrameServer for AbcTcpServer {
    fn listen_conn(&self, handler: impl Fn(TunnelConn) + Send + Sync + 'static) {
        let mut accepted_conns = Arc::new(Mutex::new(Vec::new()));
        let conn_handler = move |mut stream: TcpStream| {
            let tcp_socket = stream.try_clone().unwrap();

            let wrap_conn = TunnelConn {
                tuntype: "tcp".to_owned(),
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
        format!("tcp://{}", self.listener.local_addr().unwrap())
    }
}
