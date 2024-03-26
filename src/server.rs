use log::{info, trace, warn};
use std::cell::UnsafeCell;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use std::{io, thread};

use crate::option::BbkSerOption;
use crate::serve;
use crate::serve::{FrameServer, TunnelConn};

use crate::serializer::Serializer;
use crate::stub::{TunnelStub, VirtualStream};
use crate::transport::{self, TcpTransport, TlsTransport, Transport, WsTransport};
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

    fn get_tunnel_stub(&self, wmode: &str, tun: TunnelConn)->Option<TunnelStub> {
        // let tsport = wrap_tunnel(tunconn);
        let serizer = self.serizer.clone();
        match &wmode[..] {
            "tcp" => {
                let conn = tun.tcp_socket.unwrap().try_clone().unwrap();
                let transport = TcpTransport { conn };
                let stub_org = TunnelStub::new(Box::new(transport), serizer);
                Some(stub_org)
            }
            "tls" => {
                let conn = tun.tls_socket.unwrap();
                let transport = TlsTransport { conn:UnsafeCell::new(conn) };
                let stub_org = TunnelStub::new(Box::new(transport), serizer);
                Some(stub_org)
            }
            "ws" => {
                let conn = tun.websocket.unwrap();
                let transport = WsTransport { conn:UnsafeCell::new(conn) };
                let stub_org = TunnelStub::new(Box::new(transport), serizer);
                Some(stub_org)
            }
            _=>None
        }
     
    }
    fn listen_tunnel(&self, tun: TunnelConn) {
        
        let stub_org = self.get_tunnel_stub(&self.opts.work_mode, tun).unwrap();

        let server_stub_arc = Arc::new(stub_org);
        let server_stub_arc2 = server_stub_arc.clone();
        thread::spawn(move || server_stub_arc2.start());

        let selfshared = Arc::new(self);
        info!("exec here loop await stream===");
        loop {
            // println!("listen stream...");
            match server_stub_arc.streamch_recv.recv() {
                Ok(ret) => match ret {
                    None => {
                        break;
                    }
                    Some(stream) => {
                        let server_stub_arc1 = server_stub_arc.clone();
                        let self2 = selfshared.clone();
                        self2.handle_stream(stream, server_stub_arc1);
                    }
                },
                Err(err) => {
                    eprintln!("err:{:?}", err);
                }
            }
        }
    }

    fn listen_server(&self, mut server: Box<dyn FrameServer>) {
        thread::scope(|s| {
            loop {
                let tunnel = server.accept();
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

    fn init_server(&self) {
        if self.opts.listen_port <= 1024 && self.opts.listen_port >= 65535 {
            panic!("invalid port: {}", self.opts.listen_port);
        }
        let port = self.opts.listen_port as u16;
        let addr = format!("{}:{}", &self.opts.listen_addr, &port);
        info!("server listen on {:?} mode:{}", &addr, &self.opts.mode);
        let listener = TcpListener::bind(&addr).expect("listen addr error!");
        // let server = serve::new_abc_tcp_server(&self.opts.listen_addr, port).unwrap();
        match &self.opts.work_mode[..] {
            "tcp" => {
                let server = serve::AbcTcpServer::new(listener);
                self.listen_server(Box::new(server));
            }
            "tls" => {
                let server = serve::AbsTlsServer::new(listener,&self.opts.ssl_crt,&self.opts.ssl_key);
                self.listen_server(Box::new(server));
            }
            "ws" => {
                let server = serve::AbcWssServer::new(listener);
                self.listen_server(Box::new(server));
            }
            "h2" => {
                let server = serve::AbcHttp2Server::new(listener);
                self.listen_server(Box::new(server));
            }
            _ => {
                eprintln!("Unsupport mode");
                exit(-1);
            }
        };
    }

    pub fn bootstrap(&self) {
        self.init_server()
    }
}
