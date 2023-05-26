use std::collections::HashMap;
use std::error::Error;
use std::println;
use std::time::Duration;
use retry::{delay::Exponential, retry};
use tokio::sync::mpsc;

use crate::{proxy, utils};

use crate::option::{BbkCliOption, TunnelOpts};
use crate::proxy::ProxySocket;
use crate::utils::logger::Logger;
use crate::transport;

struct BrowserObj {
    cid: String,
    // proxy_socket: dyn ProxySocket,
    // stream_ch: mpsc::Sender<stub::Stream>,
}


pub struct BbkClient {
    opts: BbkCliOption,
    // logger: Logger,
    // tunnel_opts: TunnelOpts,
    // req_ch: mpsc::Receiver<BrowserObj>,
    // retry_count: u8,
    // tunnel_status: u8,
    // // stub_client: stub::TunnelStub,
    // // transport: dyn transport::Transport,
    // last_pong: u64,
    // browser_proxy: HashMap<String, BrowserObj>, // 线程共享变量
}

impl BbkClient {
    pub fn new(opts: BbkCliOption) -> Self {
        println!("client new====");
        BbkClient { opts: opts }
    }

    // fn setup_ws_connection(&mut self) -> Result<()> {
    //     let tun_opts = self.tunnel_opts.clone();
    //     self.logger.info(format!("creating {} tunnel", tun_opts.protocol));
    //     let result = retry(
    //         Exponential::from_millis(500) // 指数计算重试延迟
    //             .map(|x| Duration::from_millis(x))
    //             .take(5) // 最多重试5次
    //             .retry_if(|error| {
    //                 // 尝试捕获任何错误，并返回true以进行重试
    //                 eprintln!("setup tunnel failed!{:?}", error);
    //                 true
    //             }),
    //         || {
    //             self.transport = CreateTransport(tun_opts.clone())?;
    //             self.stub_client = stub::TunnelStub::new(&self.transport);
    //             self.stub_client.notify_pong(|up, down| {
    //                 self.logger.info(format!("tunnel health！ up:{}ms, down:{}ms rtt:{}ms", up, down, up + down));
    //             });
    //             self.tunnel_status = TUNNEL_OK;
    //             self.logger.debug("create tunnel success!");
    //             Ok(())
    //         },
    //     );
    //     result.context(format!("Failed to create {} tunnel", tun_opts.protocol))
    // }


    pub fn bootstrap(self) {
        let tunopts = match self.opts.tunnel_opts {
            Some(tp) => tp,
            None => panic!("missing tunnelOpts config"),
        };
        if self.opts.listen_port <= 1024 && self.opts.listen_port >= 65535 {
            panic!("invalid port: {}", self.opts.listen_port);
        }
        if self.opts.listen_http_port <= 1024 && self.opts.listen_http_port >= 65535 {
            panic!("invalid port: {}", self.opts.listen_http_port);
        }
        let port = self.opts.listen_port as u16;

        let proxy_server = proxy::new_proxy_server(&self.opts.listen_addr, port).unwrap();
        println!("Proxy server listening on {}", proxy_server.get_addr());

        proxy_server.listen_conn(|stream| {
            // Handle incoming connections here
            match proxy::socks5::new_socks5_proxy(stream) {
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
