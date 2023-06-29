use futures::FutureExt;
use retry::delay::jitter;
use retry::{delay::Exponential, retry};
use std::error::Error;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{println, thread};

use crate::serializer::Serializer;
use crate::{proxy, stub, utils};

use crate::option::{BbkCliOption, TunnelOpts};
use crate::proxy::ProxySocket;
use crate::transport::{self, create_transport, Transport};
use crate::utils::logger::{LogLevel, Logger};

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
    logger: Logger,
    tunnel_opts: TunnelOpts,
    req_recver: Receiver<BrowserObj>,
    req_sender: Sender<BrowserObj>,
    ping_sender:Sender<bool>,
    ping_recver:Receiver<bool>,
    // retry_count: u8,
    serizer: Arc<Box<Serializer>>,
    tunnel_status: u8,
    stub_client: Option<Arc<Mutex<stub::TunnelStub>>>,
    // last_pong: u64,
    // browser_proxy: HashMap<String, BrowserObj>, // 线程共享变量
}

impl BbkClient {
    pub fn new(opts: BbkCliOption) -> Self {
        println!("client new====");
        // let (tx, mut rx) = mpsc::channel(10); // 使用tokio的mpsc
        // let mut proxy_server_tx = tx.clone();
        let logger = Logger::new(LogLevel::Info);
        let tunopts = opts.tunnel_opts.clone().unwrap();
        let serizer = Serializer::new(&tunopts.method, &tunopts.password).unwrap();
        let (tx, rx) = mpsc::channel(); // 使用tokio的mpsc替代crossbeam_channel
        let (ping_tx,ping_rx) = mpsc::channel();
        BbkClient {
            tunnel_opts: tunopts,
            opts: opts,
            req_recver: rx,
            req_sender: tx,
            ping_sender:ping_tx,
            ping_recver:ping_rx,
            serizer: Arc::new(Box::new(serizer)),
            logger: logger,
            stub_client: None,
            tunnel_status: TUNNEL_INIT,
        }
    }
    fn setup_ws_connection(&mut self) -> Result<stub::TunnelStub, Box<dyn Error>> {
        let tun_opts = self.tunnel_opts.clone();
        self.logger.info(&format!("creating {} tunnel", tun_opts.protocol));
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

    pub fn service_worker(&self) {
        // let client2 = client.clone();
        // let inc2 = client.clone();
    }
    pub fn bootstrap(&mut self) {
        let opts = self.opts.clone();
        let tunopts = self.tunnel_opts.clone();

        let reqsender1 = self.req_sender.clone();

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
            println!("Proxy server listening on {}", proxy_server.get_addr());

            proxy_server.listen_conn(|tcpstream| {
                // Handle incoming connections here
                match proxy::socks5::new_socks5_proxy(tcpstream) {
                    Ok(proxy) => {
                        let addr = proxy.get_addr();
                        // println!("addr:{:?}", addr.to_vec());
                        let ret: Result<utils::socks5::AddrInfo, Box<dyn Error>> = utils::socks5::AddrInfo::from_buffer(addr);
                        if let Ok(addrinfo) = ret {
                            println!("=====await socks5...{}:{}", addrinfo.host, addrinfo.port);
                            let reqobj = BrowserObj {
                                cid: "00000000000000000000000000000000".to_owned(),
                                proxy_socket: proxy,
                            };
                            reqsender1.send(reqobj);
                        } else {
                            println!("=====exception addr socks5...");
                        }
                    }
                    Err(e) => {
                        println!("socks5 proxy err:{}", e);
                    }
                };
            });
        });

        // main loop check tunnel and reconnecting
        loop {
            thread::sleep(Duration::from_millis(1000));
            // println!("server worker start, tunnel status:{}", lock_self.tunnel_status);
            // tokio::time::sleep(Duration::from_secs(2)).await;
            if self.tunnel_status != TUNNEL_OK {
                match self.setup_ws_connection() {
                    Ok(worker) => {
                        worker.start();
                        self.tunnel_status = TUNNEL_OK;
                        let worker_arc = Arc::new(Mutex::new(worker));
                        let worker_arc2 = worker_arc.clone();
                        self.stub_client = Some(worker_arc);
                        // println!("tunnel setup success.");
                        // thread::spawn(move || loop {
                        //     thread::sleep(Duration::from_millis(3000));
                        //     worker_arc2.lock().unwrap().ping();
                        // });
                    }
                    Err(er) => {
                        eprintln!("Failed to setup ws connection: {:?}", er);
                        self.tunnel_status = TUNNEL_DISCONNECT;
                        // sleep(Duration::from_millis(1000 * 3)).await; // retry it
                        // thread::sleep(Duration::from_millis(1000 * 3));
                        continue;
                    }
                }
            }
            // println!("await request channel");
            match self.req_recver.recv() {
                Ok(mut request) => {
                    self.logger.info("handle browser request... ");
                    if let Some(stub) = &self.stub_client{
                        let stub2 = stub.lock().unwrap();
                        let st = stub2.start_stream(request.proxy_socket.get_addr());
                        println!("reqcid:{}", st.cid);
                        request.cid = st.cid.clone();
                    }
                 
                    // cli.browserProxy[st.cid] = request
                }
                Err(err) => {}
            }
        }
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
