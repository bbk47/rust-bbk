use futures::FutureExt;
use retry::delay::jitter;
use retry::{delay::Exponential, retry};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{println, thread};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::time::sleep;

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
    req_recver: mpsc::UnboundedReceiver<BrowserObj>,
    req_sender: mpsc::UnboundedSender<BrowserObj>,
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
        let logger = Logger::new(LogLevel::Info);
        let tunopts = opts.tunnel_opts.clone().unwrap();
        let serizer = Serializer::new(&tunopts.method, &tunopts.password).unwrap();
        let (tx, rx) = mpsc::unbounded_channel(); // 使用tokio的mpsc替代crossbeam_channel
        BbkClient {
            tunnel_opts: tunopts,
            opts: opts,
            req_recver: rx,
            req_sender: tx,
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

    pub fn service_worker(client: BbkClient) {
        // let client2 = client.clone();
        // let inc2 = client.clone();
    }
    pub async fn bootstrap(self) {
        let opts = self.opts.clone();
        let tunopts = self.tunnel_opts.clone();
        let cli = Arc::new(Mutex::new(self));

        let inc2 = cli.clone();
        let inc3 = cli.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_millis(100)).await;
                let mut lock_self = inc2.lock().unwrap();
                // println!("server worker start, tunnel status:{}", lock_self.tunnel_status);
                // tokio::time::sleep(Duration::from_secs(2)).await;
                if lock_self.tunnel_status != TUNNEL_OK {
                    match lock_self.setup_ws_connection() {
                        Ok(worker) => {
                            worker.start();
                            lock_self.tunnel_status = TUNNEL_OK;
                            lock_self.stub_client = Some(Arc::new(worker));
                            // println!("tunnel setup success.");
                        }
                        Err(er) => {
                            eprintln!("Failed to setup ws connection: {:?}", er);
                            lock_self.tunnel_status = TUNNEL_DISCONNECT;
                            // sleep(Duration::from_millis(1000 * 3)).await; // retry it
                            // thread::sleep(Duration::from_millis(1000 * 3));
                            continue;
                        }
                    }
                }
                // println!("await request channel");
                match lock_self.req_recver.try_recv() {
                    Ok(ref request) => {
                        lock_self.logger.info("handle browser request... ");
                        let stub = lock_self.stub_client.as_ref().unwrap();
                        let st = stub.start_stream(request.proxy_socket.get_addr());
                        println!("reqcid:{}", st.cid);
                        // cli.browserProxy[st.cid] = request
                    }
                    Err(TryRecvError::Disconnected) => {
                        println!("request channel is closed");
                    }
                    Err(TryRecvError::Empty) => {
                        // println!("request channel is empty.");
                    }
                }
            }
        });

        let _ = tokio::spawn(async move {
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
                            println!("=====await socks5...{}:{}", addrinfo.address, addrinfo.port);
                            let reqobj = BrowserObj {
                                cid: "00000000000000000000000000000000".to_owned(),
                                proxy_socket: proxy,
                            };
                            let cli = inc3.lock().unwrap();
                            cli.logger.info("send request===");
                            let _ = cli.req_sender.send(reqobj);
                        } else {
                            println!("=====exception addr socks5...");
                        }
                    }
                    Err(e) => {
                        println!("socks5 proxy err:{}", e);
                    }
                };
            });
        })
        .await;

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
