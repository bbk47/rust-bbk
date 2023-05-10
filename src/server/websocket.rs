use std::error::Error;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

struct AbcWssServer {
    addr: String,
    listener: TcpListener,
    path: String,
    http_handler: Option<fn(http::Request<()>) -> http::Response<Vec<u8>> + Send + Sync + 'static>,
    tun_handler: Option<fn(TunnelConn) + Send + Sync + 'static>,
}

impl AbcWssServer {
    fn new(host: &str, port: u16, path: &str) -> Result<Self, Box<dyn Error>> {
        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(addr)?;

        let server = Arc::new(http::server::HttpServer::new(move |req, writer| {
            let pathname = req.uri().path();
            if pathname == path {
                let ws_stream = async_tungstenite::tokio::accept_async(req.into()).unwrap();
                let ws_client = Arc::new(websocket::sync::Client::from_raw_socket(ws_stream.get_ref().0.try_clone().unwrap(), websocket::Protocol::default()).unwrap());
                let tcp_socket = ws_stream.get_ref().0;

                let wrap_conn = TunnelConn {
                    tuntype: "ws".to_owned(),
                    wsocket: ws_client,
                    tcp_socket,
                    h2_socket: h2conn::Conn::new(tcp_socket.try_clone().unwrap()).unwrap(),
                };

                if let Some(tun_handler) = &self.tun_handler {
                    tun_handler(wrap_conn);
                }
            } else if let Some(http_handler) = &self.http_handler {
                let result = http_handler(req);
                let _ = writer.send(result);
            }
        }));

        Ok(AbcWssServer {
            addr: format!("ws://{}:{}{}", host, port, path),
            listener,
            path: path.to_owned(),
            http_handler: None,
            tun_handler: None,
        })
    }
}



impl FrameServer for AbcWssServer {
    fn listen_conn(&self, handler: impl Fn(TunnelConn) + Send + Sync + 'static) {
        let mut accepted_conns = Arc::new(Mutex::new(Vec::new()));
        let conn_handler = move |mut stream: TcpStream| {
            let tcp_socket = stream.try_clone().unwrap();

            let wrap_conn = TunnelConn {
                tuntype: "wss".to_owned(),
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

    fn listen_http_conn(&mut self, handler: impl Fn(http::Request<()>) -> http::Response<Vec<u8>> + Send + Sync + 'static) {
        self.http_handler = Some(handler);
    }

    fn get_addr(&self) -> String {
        self.addr.clone()
    }
}
