use log::{error, info, trace, warn};
use retry::delay::jitter;
use retry::{delay::Exponential, retry};
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::sleep;
use std::time::Duration;
use std::{io, mem, println, thread};

use crate::serializer::Serializer;
use crate::stub::{TunnelStub, VirtualStream};
use crate::utils::{forward, get_timestamp, MyError};
use crate::{proxy, stub, utils};

use crate::option::{BbkCliOption, TunnelOpts};
use crate::proxy::{ProxyServer, ProxySocket};
use crate::transport::{self, Transport};

const TUNNEL_INIT: u8 = 0x1;
const TUNNEL_OK: u8 = 0x2;
const TUNNEL_DISCONNECT: u8 = 0x3;

struct BrowserObj {
    cid: String,
    proxy_socket: ProxySocket,
}

pub struct BbkClient {
    opts: BbkCliOption,
    browser_proxys: Arc<Mutex<HashMap<String, Arc<BrowserObj>>>>,
    tunnel_opts: TunnelOpts,
    serizer: Arc<Serializer>,
    tunnel_status: Mutex<u8>,
    stub_client: RwLock<Option<Arc<stub::TunnelStub>>>,
}

impl BbkClient {
    pub fn new(opts: BbkCliOption) -> Self {
        println!("client new====");
        // let (tx, mut rx) = mpsc::channel(10); // 使用tokio的mpsc
        // let mut proxy_server_tx = tx.clone();
        let tunopts = opts.tunnel_opts.clone().unwrap();
        let serizer = Serializer::new(&tunopts.method, &tunopts.password).unwrap();

        BbkClient {
            tunnel_opts: tunopts,
            opts: opts,
            browser_proxys: Arc::new(Mutex::new(HashMap::new())),
            serizer: Arc::new(serizer),
            stub_client: RwLock::new(None),
            tunnel_status: Mutex::new(TUNNEL_INIT),
        }
    }
    fn setup_ws_connection(&self) -> Result<stub::TunnelStub, Box<dyn Error>> {
        let tun_opts = self.tunnel_opts.clone();
        info!("creating {} tunnel", tun_opts.protocol);
        let tunport: u16 = tun_opts.port.parse()?;
        // 根据字符串构造一个 Box<dyn Error>
        let err_message: &'static str = "Something went wrong";
        let err = Box::<dyn Error>::from(MyError { message: err_message.to_string() });
        match &tun_opts.protocol[..] {
            "tcp" => {
                let ts = transport::new_tcp_transport(&tun_opts.host, tunport)?;
                let stub = stub::TunnelStub::new(ts, self.serizer.clone());
                Ok(stub)
            }
            "tls" => {
                let ts = transport::new_tls_transport(&tun_opts.host, tunport)?;
                let stub = stub::TunnelStub::new(ts, self.serizer.clone());
                Ok(stub)
            }
            "ws" => {
                let ts = transport::new_websocket_transport(&tun_opts.host, tunport, &tun_opts.path, tun_opts.secure)?;
                let stub = stub::TunnelStub::new(ts, self.serizer.clone());
                Ok(stub)
            }
            "h2" => {
                let ts = transport::new_tls_transport(&tun_opts.host, tunport)?;
                let stub = stub::TunnelStub::new(ts, self.serizer.clone());
                Ok(stub)
            }
            _ => Err(err),
        }
    }
    fn listen_stream(&self, worker: TunnelStub) {
        let worker_arc = Arc::new(worker);
        let worker_arc1 = worker_arc.clone();
        let worker_arc3 = worker_arc.clone();
        let worker_arc4 = worker_arc.clone();
        let bproxys = self.browser_proxys.clone();
        let mut stubwriter = self.stub_client.write().unwrap();
        *stubwriter = Some(worker_arc);
        mem::drop(stubwriter);
        info!("tunnel setup success.");
        let emiter = worker_arc1.emiter.clone();
        emiter.lock().unwrap().subscribe(
            "pong",
            Box::new(|message| {
                let now = get_timestamp();
                info!("tunnel health up:{}ms, down:{}ms, rtt:{}ms", message.atime - message.stime, now - message.atime, now - message.stime);
            }),
        );
        thread::spawn(move || worker_arc4.start());
        info!("listne stream====");
        loop {
            // block thread
            match worker_arc3.accept() {
                Ok(ret) => {
                    match ret {
                        None => {
                            break;
                        }
                        Some(vstream1) => {
                            info!("2.EST ===>:{}", &vstream1.addstr);
                            let cid = vstream1.cid.clone();
                            let browser_proxys = bproxys.lock().unwrap();
                            let browserobj_ret = browser_proxys.get(&cid);
                            if let Some(browser_obj) = browserobj_ret {
                                // handle brower socket to stream
                                let browser_socket1 = browser_obj.proxy_socket.conn.try_clone().unwrap();
                                thread::spawn(move || {
                                    forward(browser_socket1, vstream1.clone());
                                    info!("3.CLOSE stream:{}", &vstream1.addstr);
                                });
                            }
                        }
                    }
                }
                Err(err) => {
                    error!("err:{:?}", err);
                }
            }
        }
    }
    fn handle_request(&self, mut request: BrowserObj) {
        let addr = request.proxy_socket.get_addr();
        let stub = self.stub_client.read().unwrap();
        // println!("addr:{:?}", addr.to_vec());
        let ret: Result<utils::socks5::AddrInfo, Box<dyn Error>> = utils::socks5::AddrInfo::from_buffer(addr);
        if let Ok(addrinfo) = ret {
            info!("1.CMD {}:{}", addrinfo.host, addrinfo.port);
            if let Some(stub2) = stub.as_ref() {
                let cid = stub2.start_stream(request.proxy_socket.get_addr());
                let mut brower_proxys = self.browser_proxys.lock().unwrap();
                request.cid = cid.clone();
                brower_proxys.insert(cid, Arc::new(request));
            }
        } else {
            error!("=====exception addr socks5...");
        }
    }

    fn keep_ping(&self) {
        loop {
            // println!("keep_ping");
            // block thread
            thread::sleep(Duration::from_millis(3000));
            let stub = self.stub_client.read().unwrap();
            if let Some(stub2) = stub.as_ref() {
                stub2.ping();
            }
        }
    }
    fn service_worker(&self) {
        // main loop check tunnel and reconnecting
        loop {
            // println!("service_worker");
            let mut status = self.tunnel_status.lock().unwrap();
            // info!("server worker start, tunnel status:{}", self.tunnel_status);
            if *status != TUNNEL_OK {
                // println!("setup ws====>");
                match self.setup_ws_connection() {
                    Ok(worker) => {
                        *status = TUNNEL_OK;
                        info!("stub worker listening");
                        self.listen_stream(worker);
                        *status = TUNNEL_DISCONNECT;
                        info!("stub worker stoped...")
                    }
                    Err(er) => {
                        error!("Failed to setup ws connection: {:?}", er);
                        *status = TUNNEL_DISCONNECT;
                        sleep(Duration::from_millis(1000 * 3)); // retry it
                        continue;
                    }
                }
            }
        }
    }

    fn get_tcp_server(&self, port: i64) -> ProxyServer {
        let opts = self.opts.clone();
        if port <= 1024 && port >= 65535 {
            panic!("invalid port: {}", port);
        }
        let port2 = port as u16;
        let proxy_server = proxy::new_proxy_server(&opts.listen_addr, port2).unwrap();
        info!("Proxy server listening on {}", proxy_server.get_addr());
        return proxy_server;
    }
    fn init_socks5_server(&self) {
        let server = self.get_tcp_server(self.opts.listen_port);

        server.listen_conn(|tcpstream| {
            // Handle incoming connections here
            match proxy::socks5::new_socks5_proxy(tcpstream) {
                Ok(proxy) => {
                    let reqobj = BrowserObj {
                        cid: "00000".to_owned(),
                        proxy_socket: proxy,
                    };
                    self.handle_request(reqobj);
                    // tx.send(reqobj).expect("dispatch proxy socket error");
                }
                Err(e) => {
                    error!("new socks5 proxy err:{}", e);
                }
            };
        });
    }
    fn init_connect_server(&self) {
        let server = self.get_tcp_server(self.opts.listen_http_port);
        server.listen_conn(|tcpstream| {
            // Handle incoming connections here
            match proxy::connect::new_connect_proxy(tcpstream) {
                Ok(proxy) => {
                    let reqobj = BrowserObj {
                        cid: "00000".to_owned(),
                        proxy_socket: proxy,
                    };
                    self.handle_request(reqobj);
                    // tx.send(reqobj).expect("dispatch proxy socket error");
                }
                Err(e) => {
                    error!("new connect proxy err:{}", e);
                }
            };
        });
    }

    pub fn bootstrap(&self) {
        thread::scope(|s| {
            s.spawn(move || self.service_worker());
            s.spawn(move || self.init_socks5_server());
            s.spawn(move || self.init_connect_server());
            s.spawn(move || self.keep_ping());
        });
    }
}
