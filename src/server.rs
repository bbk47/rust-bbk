use log::{info, trace, warn};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use std::{io, thread};

use crate::option::BbkSerOption;
use crate::serve;
use crate::serve::{FrameServer, TunnelConn};

use crate::serializer::Serializer;
use crate::stub::{TunnelStub, VirtualStream};
use crate::transport::{self, TcpTransport, Transport};
use crate::utils::socks5::AddrInfo;

pub struct BbkServer {
    opts: BbkSerOption,
    serizer: Arc<Box<Serializer>>,
}

impl BbkServer {
    pub fn new(opts: BbkSerOption) -> Self {
        info!("server new====");
        let serizer = Serializer::new(&opts.method, &opts.password).unwrap();
        BbkServer {
            opts: opts,
            serizer: Arc::new(Box::new(serizer)),
        }
    }

    fn forward(&mut self, stream: Arc<VirtualStream>, socket: TcpStream) {
        let mut socket_writer = socket.try_clone().unwrap();
        let mut socket_reader = socket.try_clone().unwrap();

        info!("handle stream  to remote");
        let mut v_stream1 = stream.try_clone().unwrap();
        let mut v_stream2 = stream.try_clone().unwrap();
        thread::spawn(move || {
            let ret = io::copy(&mut v_stream1, &mut socket_writer);
            match ret {
                Ok(_) => {
                    info!("copy stream to  remote socket complete.");
                    socket.shutdown(std::net::Shutdown::Both).expect("shutdown socket err");
                }
                Err(err) => {
                    info!("copy stream to  remote socket error:{:?}", err);
                }
            }
        });
        // let stub2 = server_stub_arc2.clone();
        let ret = io::copy(&mut socket_reader, &mut v_stream2);
        match ret {
            Ok(_) => {
                info!("copy remote socket to stream complete.");
                v_stream2.close_peer();
            }
            Err(err) => {
                info!("copy remote socket to stream error:{:?}", err);
            }
        }
    }

    fn listenTunnel(&self, tun: TunnelConn) {
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

            info!("exec here loop await stream===");
            loop {
                // println!("listen stream...");
                match server_stub_arc2.streamch_recv.recv() {
                    Ok(stream) => {
                        println!("stream ===> addr:{:?}", &stream.addstr);
                        let addrinfo = AddrInfo::from_buffer(&stream.addr).unwrap();
                        info!("REQ CONNECT=>{}\n", &stream.addstr);
                        let socketaddr = (addrinfo.host, addrinfo.port).to_socket_addrs();
                        if let Ok(mut socketaddr2) = socketaddr {
                            let socket_addr = socketaddr2.next().unwrap();
                            let socket_addr2 = socket_addr.clone();
                            let stub_clone = server_stub_arc2.clone();
                            thread::spawn(move || {
                                let socket_addr2 = socket_addr2;
                                let conn = TcpStream::connect_timeout(&socket_addr2, Duration::from_secs(10));
                                if let Ok(socket) = conn {
                                    info!("DIAL SUCCESS==>{}\n", &stream.addstr);
                                    stub_clone.set_ready(&stream);
                                    self.forward(stream, socket);
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

    fn init_server(&self) {
        if self.opts.listen_port <= 1024 && self.opts.listen_port >= 65535 {
            panic!("invalid port: {}", self.opts.listen_port);
        }
        let port = self.opts.listen_port as u16;
        let addr = format!("{}:{}", &self.opts.listen_addr, &port);
        // let server = serve::new_abc_tcp_server(&self.opts.listen_addr, port).unwrap();
        let server = serve::AbcTcpServer::new(&self.opts.listen_addr, port).unwrap();

        info!("server listen on {:?}", server.get_addr());
        // 这里是需要异步执行的代码
        for tunnel in server {
            match tunnel {
                Ok(tun) => {
                    // 对新连接进行处理
                    // 处理完成后关闭连接
                    println!("new connection coming...");
                    self.listenTunnel(tun);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }

    pub fn bootstrap(&self) {
        self.init_server()
    }
}
