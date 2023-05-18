
use std::println;

use crate::proxy;

use crate::option::BbkCliOption;
use crate::proxy::socks5::ProxySocket;

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
        let port = self.opts.listen_port as u16;

        let proxy_server = proxy::new_proxy_server(&self.opts.listen_addr, port).unwrap();
        println!("Proxy server listening on {}", proxy_server.get_addr());

        proxy_server.listen_conn(|stream| {
            // Handle incoming connections here
            let socketproxy = proxy::socks5::new_socks5_proxy(stream).unwrap();
            let addr = socketproxy.get_addr();
            println!("=====await connect")
        });
    }
}
