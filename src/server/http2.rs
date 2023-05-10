// use std::error::Error;
// use std::net::{TcpListener, TcpStream};
// use std::sync::{Arc, Mutex};

// struct AbcHttp2Server {
//     addr: String,
//     ssl_crt: String,
//     ssl_key: String,
//     listener: TcpListener,
//     path: String,
//     http_handler: Option<fn(http::Request<()>) -> http::Response<Vec<u8>> + Send + Sync + 'static>,
//     tun_handler: Option<fn(TunnelConn) + Send + Sync + 'static>,
// }

// impl AbcHttp2Server {
//     fn new(host: &str, port: u16, path: &str, ssl_crt: String, ssl_key: String) -> Result<Self, Box<dyn Error>> {
//         let addr = format!("{}:{}", host, port);
//         let listener = TcpListener::bind(addr)?;

//         let server = Arc::new(http::server::HttpServer::new(move |req, writer| {
//             let pathname = req.uri().path();
//             if pathname == path {
//                 // We only accept HTTP/2!
//                 // (Normally it's quite common to accept HTTP/1.- and HTTP/2 together.)
//                 let tcp_stream = writer.get_ref().clone();
//                 let h2_conn = h2conn::Conn::new(tcp_stream.try_clone().unwrap()).unwrap();
//                 let h2_client = Arc::new(httpbis::Client::new(h2_conn.clone()));
//                 let wrap_conn = TunnelConn {
//                     tuntype: "h2".to_owned(),
//                     wsocket: Arc::new(websocket::sync::Client::new(&TcpStream::from(h2_conn)).unwrap()),
//                     tcp_socket: tcp_stream,
//                     h2_socket: h2_conn,
//                 };

//                 if let Some(tun_handler) = &self.tun_handler {
//                     tun_handler(wrap_conn);
//                 }
//             } else if let Some(http_handler) = &self.http_handler {
//                 let result = http_handler(req);
//                 let _ = writer.send(result);
//             }
//         }));

//         Ok(AbcHttp2Server {
//             addr: format!("https://{}:{}{}", host, port, path),
//             ssl_crt,
//             ssl_key,
//             listener,
//             path: path.to_owned(),
//             http_handler: None,
//             tun_handler: None,
//         })
//     }
// }

// trait FrameServer {
//     fn listen_conn(&self, handler: impl Fn(TunnelConn) + Send + Sync + 'static);
//     fn listen_http_conn(&mut self, handler: impl Fn(http::Request<()>) -> http::Response<Vec<u8>> + Send + Sync + 'static);
//     fn get_addr(&self) -> String;
// }

// impl FrameServer for AbcHttp2Server {
//     fn listen_conn(&self, handler: impl Fn(TunnelConn) + Send + Sync + 'static) {
//         let mut accepted_conns = Arc::new(Mutex::new(Vec::new()));
//         let conn_handler = move |mut stream: TcpStream| {
//             let tcp_socket = stream.try_clone().unwrap();

//             let wrap_conn = TunnelConn {
//                 tuntype: "h2".to_owned(),
//                 wsocket: Arc::new(websocket::sync::Client::new(&stream).unwrap()),
//                 tcp_socket,
//                 h2_socket: h2conn::Conn::new(stream.try_clone().unwrap()).unwrap(),
//             };

//             let conn_handler = handler.clone();
//             let accepted_conns = accepted_conns.clone();

//             std::thread::spawn(move || {
//                 accepted_conns.lock().unwrap().push(tcp_socket.try_clone().unwrap());
//                 conn_handler(wrap_conn);
//             });
//         };

//         for conn in self.listener.incoming() {
//             if let Ok(stream) = conn {
//                 let conn_handler = conn_handler.clone();

//                 std::thread::spawn(move || conn_handler(stream));
//             }
//         }
//     }

//     fn listen_http_conn(&mut self, handler: impl Fn(http::Request<()>) -> http::Response<Vec<u8>> + Send + Sync + 'static) {
//         self.http_handler = Some(handler);
//     }

//     fn get_addr(&self) -> String {
//         self.addr.clone()
//     }
// }
