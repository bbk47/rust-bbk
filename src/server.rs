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
use crate::utils::forward;
use crate::utils::socks5::AddrInfo;

pub struct BbkServer {
    opts: BbkSerOption,
    serizer: Arc<Serializer>,
}

impl BbkServer {
    pub fn new(opts: BbkSerOption) -> Self {
        info!("server new====");
        let serizer = Serializer::new(&opts.method, &opts.password).unwrap();
        BbkServer {
            opts: opts,
            serizer: Arc::new(serizer),
        }
    }

    fn handle_stream(&self, stream: Arc<VirtualStream>, stub: Arc<TunnelStub>) {
        // info!("stream ===> addr:{}", &stream.addstr);
        let addrstr = stream.addstr.clone();
        let addrinfo = AddrInfo::from_buffer(&stream.addr).unwrap();
        info!("REQ CONNECT=>{}", &stream.addstr);
        let socketaddr = (addrinfo.host, addrinfo.port).to_socket_addrs();
        if let Ok(mut socketaddr2) = socketaddr {
            let socket_addr = socketaddr2.next().unwrap();
            let socket_addr2 = socket_addr.clone();
            let stub_clone = stub.clone();
            thread::spawn(move || {
                let socket_addr2 = socket_addr2;
                let conn = TcpStream::connect_timeout(&socket_addr2, Duration::from_secs(10));
                if let Ok(socket) = conn {
                    info!("DIAL SUCCESS==>{}", &addrstr);
                    stub_clone.set_ready(&stream);
                    forward(socket, stream);
                    info!("CLOSE stream:{}", &addrstr);
                }
            });
        }
    }

    fn listen_tunnel(&self, tun: TunnelConn) {
        let serizer = self.serizer.clone();
        // let tsport = wrap_tunnel(tunconn);
        let conn = tun.tcp_socket.try_clone().unwrap();
        info!("tsport:{:?}", &conn);
        let transport = TcpTransport { conn };
        // let tsport: Arc<Box<dyn Transport + Send + Sync>> = Arc::new(Box::new(tcpts));
        let stub_org = TunnelStub::new(Box::new(transport), serizer);
        let server_stub_arc = Arc::new(stub_org);
        let server_stub_arc2 = server_stub_arc.clone();
        thread::spawn(move||server_stub_arc2.start());

        let selfshared = Arc::new(self);
        info!("exec here loop await stream===");
        loop {
            // println!("listen stream...");
            match server_stub_arc.streamch_recv.recv() {
                Ok(ret) => {
                    match ret {
                        None=>{
                            break;
                        }
                        Some(stream)=>{
                            let server_stub_arc1 = server_stub_arc.clone();
                            let self2 = selfshared.clone();
                            self2.handle_stream(stream, server_stub_arc1);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("err:{:?}", err);
                }
            }
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

        info!("server listen on {:?}", server.get_addr());
        thread::scope(|s| {
            // 这里是需要异步执行的代码
            for tunnel in server {
                match tunnel {
                    Ok(tun) => {
                        // 对新连接进行处理
                        // 处理完成后关闭连接
                        println!("new connection coming...");
                        s.spawn(move || self.listen_tunnel(tun));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
        });
    }

    pub fn bootstrap(&self) {
        self.init_server()
    }
}
