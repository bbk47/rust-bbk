use retry::delay::jitter;
use retry::{delay::Exponential, retry};
use std::collections::HashMap;
use std::error::Error;
use std::println;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::serializer::Serializer;
use crate::{proxy, utils, stub};

use crate::option::{BbkCliOption, TunnelOpts};
use crate::proxy::ProxySocket;
use crate::transport::{self, create_transport};
use crate::utils::logger::{Logger, LogLevel};

const TUNNEL_INIT: u8 = 0x1;
const TUNNEL_OK: u8 = 0x2;
const TUNNEL_DISCONNECT: u8 = 0x3;

struct BrowserObj {
    cid: String,
    proxy_socket: ProxySocket,
    // stream_ch: mpsc::Sender<stub::Stream>,
}

pub struct BbkClient<'a> {
    opts: BbkCliOption,
    logger: Logger,
    tunnel_opts: TunnelOpts,
    req_ch: mpsc::UnboundedReceiver<BrowserObj>,
    // retry_count: u8,
    serizer: Arc<Serializer>,
    tunnel_status: u8,
    stub_client: Option<Arc<stub::TunnelStub<'a>>>,
    // last_pong: u64,
    // browser_proxy: HashMap<String, BrowserObj>, // 线程共享变量
}

impl<'a> BbkClient<'a> {
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
            req_ch: rx,
            serizer: Arc::new(serizer),
            logger: logger,
            stub_client:None,
            tunnel_status: TUNNEL_INIT,
        }
    }
    fn setup_ws_connection(&'a mut self) -> Result<(),Box<dyn Error>> {
        let tun_opts = self.tunnel_opts.clone();
        self.logger.info(&format!("creating {} tunnel", tun_opts.protocol));
        let result: Result<(), retry::Error<_>> = retry(Exponential::from_millis(10).map(jitter).take(3),|| {
            match create_transport(&tun_opts) {
                Ok(tsport) => {
                    let arcts = Arc::new(tsport);
                   let stub: stub::TunnelStub<'_> = stub::TunnelStub::new(arcts,&self.serizer);
                   self.stub_client = Some(Arc::new(stub));
                    // self.stub_client.notify_pong(|up, down| {
                    //     self.logger.info(&format!("tunnel health！ up:{}ms, down:{}ms rtt:{}ms", up, down, up + down));
                    // });
                    self.tunnel_status = TUNNEL_OK;
                    self.logger.debug("create tunnel success!");
                    return Ok(());
                },
                Err(err) => {
                    self.logger.error(&format!("Failed to create {} tunnel: {:?}", tun_opts.protocol, err));
                    return Err(err);
                }
            }

        });
        
       if result.is_err() {
            self.logger.error(&format!("Failed to create {} tunnel: {:?}", tun_opts.protocol, result.err()));
            self.tunnel_status = TUNNEL_DISCONNECT;
            return Err("Failed to create tunnel".into());
       }
       Ok(())
    }

    fn service_worker(&mut self) {
        // let (tx, mut rx) = mpsc::channel(10); // 使用tokio的mpsc
        // let mut proxy_server_tx: mpsc::Sender<BrowserObj> = tx.clone();
        // tokio::spawn(async {
        //     loop {
        //         if self.tunnel_status != TUNNEL_OK {
        //             if let Err(err) = self.setup_ws_connection() {
        //                 eprintln!("Failed to setup ws connection: {:?}", err);
        //                 self.tunnel_status = TUNNEL_DISCONNECT;
        //                 continue;
        //             }
        //         }
        //         // match self.req_ch.recv().await {
        //         //     Some(ref request) => {
        //         //         self.logger.info("handle browser request... ")
        //         //         // let proxy_socket = request.proxy_socket;
        //         //         // let st = self.stub_client.StartStream(proxy_socket.GetAddr());
        //         //         // cli.browserProxy[st.Cid] = request
        //         //     }
        //         //     None => (),
        //         // }
        //     }
        // });
    }
    pub fn bootstrap(&mut self) {
        self.service_worker();
        let tunopts = self.tunnel_opts.clone();
        if self.opts.listen_port <= 1024 && self.opts.listen_port >= 65535 {
            panic!("invalid port: {}", self.opts.listen_port);
        }
        if self.opts.listen_http_port <= 1024 && self.opts.listen_http_port >= 65535 {
            panic!("invalid port: {}", self.opts.listen_http_port);
        }
        let port = self.opts.listen_port as u16;

        let proxy_server = proxy::new_proxy_server(&self.opts.listen_addr, port).unwrap();
        println!("Proxy server listening on {}", proxy_server.get_addr());

        proxy_server.listen_conn(|tcpstream| {
            // Handle incoming connections here
            match proxy::socks5::new_socks5_proxy(tcpstream) {
                Ok(proxy) => {
                    let addr = proxy.get_addr();
                    // println!("addr:{:?}", addr.to_vec());
                    let ret: Result<utils::socks5::AddrInfo, Box<dyn Error>> = utils::socks5::AddrInfo::from_buffer(addr);
                    if let Ok(addrinfo) = ret {
                        println!("=====await socks5...{}:{}", addrinfo.address, addrinfo.port)
                    } else {
                        println!("=====exception addr socks5...");
                    }
                }
                Err(e) => {
                    println!("socks5 proxy err:{}", e);
                }
            };
        });

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
