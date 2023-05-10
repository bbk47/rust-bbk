use std::net::{TcpListener, TcpStream};

struct ProxyServer {
    addr: String,
    ln: TcpListener,
}

impl ProxyServer {
    fn listen_conn<F>(&self, mut handler: F)
    where
        F: FnMut(TcpStream),
    {
        for stream in self.ln.incoming() {
            if let Ok(stream) = stream {
                handler(stream);
            }
        }
    }

    fn get_addr(&self) -> &str {
        &self.addr
    }
}

fn new_proxy_server(host: &str, port: u16) -> std::io::Result<ProxyServer> {
    let address = format!("{}:{}", host, port);
    let listener = TcpListener::bind(address)?;
    let addr_str = format!("tcp://{}", listener.local_addr()?);
    Ok(ProxyServer {
        addr: addr_str,
        ln: listener,
    })
}
