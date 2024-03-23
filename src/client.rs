use futures::FutureExt;
use log::{error, info, trace, warn};
use retry::delay::jitter;
use retry::{delay::Exponential, retry};
use std::collections::HashMap;
use std::error::Error;
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use std::{io, println, thread};

use crate::serializer::Serializer;
use crate::stub::{TunnelStub, VirtualStream};
use crate::utils::forward;
use crate::{proxy, stub, utils};

use crate::option::{BbkCliOption, TunnelOpts};
use crate::proxy::ProxySocket;
use crate::transport::{self, create_transport, Transport};

const TUNNEL_INIT: u8 = 0x1;
const TUNNEL_OK: u8 = 0x2;
const TUNNEL_DISCONNECT: u8 = 0x3;

struct BrowserObj {
    cid: String,
    proxy_socket: ProxySocket,
    // stream_ch: mpsc::Sender<stub::Stream>,
}

pub struct BbkClient {
    opts: BbkCliOption,
    browser_proxys: Arc<Mutex<HashMap<String, Arc<BrowserObj>>>>,
    tunnel_opts: TunnelOpts,
    // ping_sender: Sender<bool>,
    // ping_recver: Receiver<bool>,
    // retry_count: u8,
    serizer: Arc<Box<Serializer>>,
    tunnel_status: u8,
    stub_client: Option<Arc<stub::TunnelStub>>,
    // last_pong: u64,
    // browser_proxy: HashMap<String, BrowserObj>, // 线程共享变量
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
            // ping_sender: ping_tx,
            // ping_recver: ping_rx,
            serizer: Arc::new(Box::new(serizer)),
            stub_client: None,
            tunnel_status: TUNNEL_INIT,
        }
    }
    fn setup_ws_connection(&mut self) -> Result<stub::TunnelStub, Box<dyn Error>> {
        let tun_opts = self.tunnel_opts.clone();
        info!("creating {} tunnel", tun_opts.protocol);
        let result: Result<stub::TunnelStub, retry::Error<_>> = retry(Exponential::from_millis(10).map(jitter).take(3), || match create_transport(&tun_opts) {
            Ok(tsport) => {
                let stub: stub::TunnelStub = stub::TunnelStub::new(tsport, self.serizer.clone());
                return Ok(stub);
            }
            Err(err) => {
                return Err(err);
            }
        });

        if result.is_err() {
            return Err("Failed to create tunnel".into());
        }
        Ok(result.unwrap())
    }
    pub fn listen_stream(&mut self, worker: TunnelStub) {
        let worker_arc = Arc::new(worker);
        let worker_arc3 = worker_arc.clone();
        // let proxys = self.browser_proxys.clone();
        self.stub_client = Some(worker_arc);
        info!("tunnel setup success.");
        // thread::spawn(move || loop {
        //     // block thread
        //     thread::sleep(Duration::from_millis(3000));
        //     worker_arc2.ping();
        // });
        let bproxys = self.browser_proxys.clone();
        thread::spawn(move || loop {
            // block thread
            info!("listen stream accept.");
            match worker_arc3.accept() {
                Ok(stream) => {
                    info!("request ===>:{:?}", &stream.addstr);
                    let cid = stream.cid.clone();
                    let browser_proxys =  bproxys.lock().unwrap();
                    let browserobj_ret = browser_proxys.get(&cid);
                    if let Some(browser_obj) = browserobj_ret {
                        // handle brower socket to stream
                        let browser_socket1 = browser_obj.proxy_socket.conn.try_clone().unwrap();
                        forward(browser_socket1, stream)
                    }
                }
                Err(err) => {
                    error!("err:{:?}", err);
                }
            }
        });
    }
    pub fn handle_request(&mut self, rx: &Receiver<BrowserObj>) {
        let browser_proxys1 = self.browser_proxys.clone();
        let stubcli = self.stub_client.clone();
        // block thread
        match rx.recv() {
            Ok(mut request) => {
                info!("handle request comming");
                if let Some(stub) = &stubcli {
                    let cid = stub.start_stream(request.proxy_socket.get_addr());
                    let mut brower_proxys = browser_proxys1.lock().unwrap();
                    request.cid = cid.clone();
                    brower_proxys.insert(cid, Arc::new(request));
                }
            }
            Err(err) => {
                println!("req_recver err:{:?}", err);
            }
        }
    }
    pub fn service_worker(&mut self, rx: Receiver<BrowserObj>) {
        // main loop check tunnel and reconnecting
        loop {
            info!("server worker start, tunnel status:{}", self.tunnel_status);
            if self.tunnel_status != TUNNEL_OK {
                match self.setup_ws_connection() {
                    Ok(worker) => {
                        self.tunnel_status = TUNNEL_OK;
                        self.listen_stream(worker);
                        info!("setup ws worker ok");
                    }
                    Err(er) => {
                        error!("Failed to setup ws connection: {:?}", er);
                        self.tunnel_status = TUNNEL_DISCONNECT;
                        sleep(Duration::from_millis(1000 * 3)); // retry it
                        continue;
                    }
                }
            }
            self.handle_request(&rx);
        }
    }

    pub fn init_proxy_server(&mut self, tx: Sender<BrowserObj>) {
        let opts = self.opts.clone();

        thread::spawn(move || {
            println!("exec here...");
            if opts.listen_port <= 1024 && opts.listen_port >= 65535 {
                panic!("invalid port: {}", opts.listen_port);
            }
            if opts.listen_http_port <= 1024 && opts.listen_http_port >= 65535 {
                panic!("invalid port: {}", opts.listen_http_port);
            }
            let port = opts.listen_port as u16;

            let proxy_server = proxy::new_proxy_server(&opts.listen_addr, port).unwrap();
            info!("Proxy server listening on {}", proxy_server.get_addr());

            proxy_server.listen_conn(|tcpstream| {
                // Handle incoming connections here
                match proxy::socks5::new_socks5_proxy(tcpstream) {
                    Ok(proxy) => {
                        let addr = proxy.get_addr();
                        // println!("addr:{:?}", addr.to_vec());
                        let ret: Result<utils::socks5::AddrInfo, Box<dyn Error>> = utils::socks5::AddrInfo::from_buffer(addr);
                        if let Ok(addrinfo) = ret {
                            info!("=====await socks5...{}:{}", addrinfo.host, addrinfo.port);
                            let reqobj = BrowserObj {
                                cid: "00000".to_owned(),
                                proxy_socket: proxy,
                            };
                            tx.send(reqobj).expect("dispatch proxy socket error");
                        } else {
                            error!("=====exception addr socks5...");
                        }
                    }
                    Err(e) => {
                        error!("socks5 proxy err:{}", e);
                    }
                };
            });
        });
    }

    pub fn bootstrap(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.init_proxy_server(tx);
        self.service_worker(rx);

        // let httpport = self.opts.listen_http_port as u16;

        // let proxy_server2 = proxy::new_proxy_server(&self.opts.listen_addr, httpport).unwrap();
        // println!("Proxy server listening on {}", proxy_server2.get_addr());

        // proxy_server2.listen_conn(|stream| {
        //     // Handle incoming connections here
        //     match proxy::connect::new_connect_proxy(stream) {
        //         Ok(proxy) => {
        //             let addr = proxy.get_addr();
        //             let ret: Result<utils::socks5::AddrInfo, Box<dyn Error>> = utils::socks5::AddrInfo::from_buffer(addr);
        //             if let Ok(addrinfo) = ret {
        //                 println!("=====await connect...{}:{}", addrinfo.address, addrinfo.port)
        //             } else {
        //                 println!("=====exception addr connect...");
        //             }
        //         }
        //         Err(e) => {
        //             println!("connect proxy err:{}", e);
        //         }
        //     };
        // });
    }
}
