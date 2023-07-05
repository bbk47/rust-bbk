use std::error::Error;
use std::fmt::Debug;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use std::{io, thread};

use crate::option::BbkSerOption;
use crate::serve;
use crate::serve::{FrameServer, TunnelConn};
use crate::utils::logger::{LogLevel, Logger};

use crate::serializer::Serializer;
use crate::stub::TunnelStub;
use crate::transport::{self, TcpTransport, Transport};
use crate::utils::socks5::AddrInfo;

pub struct BbkServer {
    opts: BbkSerOption,
    logger: Arc<Logger>,
    serizer: Arc<Box<Serializer>>,
}

impl BbkServer {
    pub fn new(opts: BbkSerOption) -> Self {
        println!("server new====");
        let logger = Logger::new(LogLevel::Info);
        let serizer = Serializer::new(&opts.method, &opts.password).unwrap();
        BbkServer {
            opts: opts,
            logger: Arc::new(logger),
            serizer: Arc::new(Box::new(serizer)),
        }
    }


    fn init_server(&self) {
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
                    let logger2 = self.logger.clone();
                    let serizer = self.serizer.clone();
                    thread::spawn(move || {
                        println!("connection====");
                        // let tsport = wrap_tunnel(tunconn);
                        let conn = tun.tcp_socket.try_clone().unwrap();
                        println!("tsport:{:?}", &conn);
                        let tcpts = TcpTransport { conn };
                        // let tsport: Arc<Box<dyn Transport + Send + Sync>> = Arc::new(Box::new(tcpts));
                        let server_stub = TunnelStub::new(Box::new(tcpts), serizer);

                        let server_stub_arc = Arc::new(server_stub);
                        let server_stub_arc1 = server_stub_arc.clone();
                        let server_stub_arc2 = server_stub_arc.clone();

                        println!("exec here loop await stream===");
                        loop {
                            // println!("listen stream...");
                            match server_stub_arc2.streamch_recv.recv() {
                                Ok(stream) => {
                                    println!("stream ===> addr:{:?}", &stream.addstr);
                                    let addrinfo = AddrInfo::from_buffer(&stream.addr).unwrap();
                                    logger2.info(&format!("REQ CONNECT=>{}\n", &stream.addstr));
                                    let socketaddr = (addrinfo.host, addrinfo.port).to_socket_addrs();
                                    if let Ok(mut socketaddr2) = socketaddr {
                                        let socket_addr = socketaddr2.next().unwrap();
                                        let socket_addr2 = socket_addr.clone();
                                        let stub_clone = server_stub_arc2.clone();
                                        thread::spawn(move || {
                                            let socket_addr2 = socket_addr2;
                                            let conn = TcpStream::connect_timeout(&socket_addr2, Duration::from_secs(10));
                                            if let Ok(socket) = conn {
                                                println!("DIAL SUCCESS==>{}", &stream.addstr);
                                                stub_clone.set_ready(&stream);
                                                let mut socket_writer = socket.try_clone().unwrap();
                                                let mut socket_reader = socket.try_clone().unwrap();
                                    
                                                println!("handle stream  to remote");
                                                let mut v_stream1 = stream.try_clone().unwrap();
                                                let mut v_stream2 = stream.try_clone().unwrap();
                                                thread::spawn(move || {
                                                    let ret = io::copy(&mut v_stream1, &mut socket_writer);
                                                    match ret {
                                                        Ok(_) => {
                                                            println!("copy stream to  remote socket complete.");
                                                            socket.shutdown(std::net::Shutdown::Both).expect("shutdown socket err");
                                                        }
                                                        Err(err) => {
                                                            println!("copy stream to  remote socket error:{:?}", err);
                                                        }
                                                    }
                                                });
                                                // let stub2 = server_stub_arc2.clone();
                                                let ret = io::copy(&mut socket_reader, &mut v_stream2);
                                                match ret {
                                                    Ok(_) => {
                                                        println!("copy remote socket to stream complete.");
                                                        v_stream2.close_peer();
                                                    }
                                                    Err(err) => {
                                                        println!("copy remote socket to stream error:{:?}", err);
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }
                                Err(err) => {
                                    eprintln!("err:{:?}", err);
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

    pub fn bootstrap(&self) {
        self.init_server()
    }
}
