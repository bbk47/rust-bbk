use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tokio::runtime::Runtime;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::time::sleep;

use crate::option::BbkSerOption;
use crate::serve;
use crate::serve::{FrameServer, TunnelConn};
use crate::utils::logger::{LogLevel, Logger};

use crate::serializer::Serializer;
use crate::stub::TunnelStub;
use crate::transport::{self, TcpTransport, Transport};

pub struct BbkServer {
    opts: BbkSerOption,
    logger: Logger,
    serizer: Arc<Box<Serializer>>,
}

impl BbkServer {
    pub fn new(opts: BbkSerOption) -> Self {
        println!("server new====");
        let logger = Logger::new(LogLevel::Info);
        let serizer = Serializer::new(&opts.method, &opts.password).unwrap();
        BbkServer {
            opts: opts,
            logger,
            serizer: Arc::new(Box::new(serizer)),
        }
    }

    async fn init_server(&self) {
        if self.opts.listen_port <= 1024 && self.opts.listen_port >= 65535 {
            panic!("invalid port: {}", self.opts.listen_port);
        }
        let port = self.opts.listen_port as u16;
        let addr = format!("{}:{}", &self.opts.listen_addr, &port);
        // let server = serve::new_abc_tcp_server(&self.opts.listen_addr, port).unwrap();
        let server = serve::AbcTcpServer::new(&self.opts.listen_addr, port).unwrap();

        // 这里是需要异步执行的代码
        for tunnel in server {
            match tunnel {
                Ok(tun) => {
                    // 对新连接进行处理
                    // 处理完成后关闭连接
                    println!("new connection coming...");
                    let serizer = self.serizer.clone();
                    tokio::spawn(async move {
                        println!("connection====");
                        // let tsport = wrap_tunnel(tunconn);
                        let conn = tun.tcp_socket.try_clone().unwrap();
                        println!("tsport:{:?}", &conn);
                        let tcpts = TcpTransport { conn };
                        // let tsport: Arc<Box<dyn Transport + Send + Sync>> = Arc::new(Box::new(tcpts));
                        let mut server_stub = TunnelStub::new(Box::new(tcpts), serizer);

                        server_stub.start();

                        println!("exec here loop await stream===");
                        loop {
                            // println!("listen stream...");
                            // thread::sleep(Duration::from_millis(1000));
                            sleep(Duration::from_millis(1000)).await;
                            match server_stub.accept() {
                                Ok(stream) => {
                                    println!("addr:{:?}", &stream.addr)
                                    // let remote_address = parse_addr_info(&stream.addr)
                                    //     .map(|info| format!("{}:{}", info.addr, info.port))
                                    //     .unwrap_or_else(|_| "unknown".into());
                                    // self.logger.info(&format!("REQ CONNECT=>{}\n", remote_address));
                                    // let target_socket = TcpStream::connect(remote_address.clone()).await;
                                    // if let Ok(socket) = target_socket {
                                    //     self.logger.info(&format!("DIAL SUCCESS==>{}", remote_address));

                                    //     server_stub.set_ready(stream);

                                    //     tokio::spawn(async move {
                                    //         if let Err(err) = stream.clone().forward(&mut socket).await {
                                    //             self.logger.error(&format!("stream error:{}", err));
                                    //         }
                                    //     });
                                    //     tokio::spawn(async move {
                                    //         if let Err(err) = socket.clone().forward(&mut stream).await {
                                    //             self.logger.error(&format!("stream error:{}", err));
                                    //         }
                                    //     });
                                    // }
                                }
                                Err(TryRecvError::Disconnected) => {
                                    break;
                                }
                                Err(TryRecvError::Empty) => {
                                    //
                                    // println!("empty===")
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }

        // self.logger.info(&format!("server listen on {:?}", server.get_addr()));
        // server.listen_conn(|t: &TunnelConn| self.handle_connection(t));
        // server .listen_conn(|tunnel_conn| {
        //     println!("Received a connection from {}", tunnel_conn.tcp_socket.peer_addr().unwrap());
        // }).unwrap();
    }

    // fn init_serizer(&self) -> Result<Serializer, Box<dyn Error>> {
    //     Serializer::new(&self.opts.method, &self.opts.password)
    //         .map_err(|e| -> Box<dyn Error> { Box::new(e) })
    // }

    pub async fn bootstrap(&self) {
        self.init_server().await
    }
}
