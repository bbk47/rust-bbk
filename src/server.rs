use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;

use tokio::runtime::Runtime;

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

    pub async fn handle_connection(&self, tunconn: &TunnelConn) {
        println!("connection====");
        // let tsport = wrap_tunnel(tunconn);
        let conn = tunconn.tcp_socket.try_clone().unwrap();
        println!("tsport:{:?}", &conn);
        let tcpts = TcpTransport { conn };
        // let tsport: Arc<Box<dyn Transport + Send + Sync>> = Arc::new(Box::new(tcpts));
        let mut server_stub = TunnelStub::new(Box::new(tcpts), self.serizer.clone());
        println!("exec here loop await stream===");
        loop {
            match server_stub.accept().await {
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
                Err(err) => {
                    self.logger.error(&format!("couldn't get a client stream: {}", err));
                    return;
                }
            }
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

        // loop {
        //     let stream = server.next();
        //     match stream {
        //         Ok(tun)=>{
        //             println!("new connection coming...");
        //         }
        //         Err(e)=>{
        //             eprintln!("Error: {}", e);
        //         }
        //     }
        //     // 处理新连接的代码
        // }

        // 这里是需要异步执行的代码
        for tunnel in server {
            match tunnel {
                Ok(tun) => {
                    // 对新连接进行处理
                    // 处理完成后关闭连接
                    println!("new connection coming...");
                    self.handle_connection(&tun).await;
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
