use std::net::{TcpListener, TcpStream};

mod base;
pub mod connect;    // 查找当前目录下的connect.rs或者connect目录下的mod.rs
pub mod socks5;    // 查找当前目录下的socks5.rs或者socks5目录下的mod.rs

pub use base::ProxySocket;

pub struct ProxyServer {
    addr: String,
    ln: TcpListener,
}

impl ProxyServer {
    pub fn listen_conn<F>(&self, mut handler: F)
    where
        F: FnMut(TcpStream),
    {
        for stream in self.ln.incoming() {
            if let Ok(stream) = stream {
                handler(stream);
            }
        }
    }

   pub fn get_addr(&self) -> &str {
        &self.addr
    }
}

pub fn new_proxy_server(host: &str, port: u16) -> std::io::Result<ProxyServer> {
    let address = format!("{}:{}", host, port);
    let listener = TcpListener::bind(address)?;
    let addr_str = format!("tcp://{}", listener.local_addr()?);
    Ok(ProxyServer {
        addr: addr_str,
        ln: listener,
    })
}
