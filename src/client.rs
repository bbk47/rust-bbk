use std::error::Error;
use std::println;

use crate::{proxy, utils};

use crate::option::BbkCliOption;
use crate::proxy::ProxySocket;

pub struct BbkClient {
    opts: BbkCliOption,
}

impl BbkClient {
    pub fn new(opts: BbkCliOption) -> Self {
        println!("client new====");
        BbkClient { opts: opts }
    }

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
