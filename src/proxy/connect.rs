use std::io::{self, BufRead, Write};
use std::net::{TcpStream, ToSocketAddrs};

struct ConnectProxy {
    addr_buf: Vec<u8>,
    conn: TcpStream,
}

impl ProxySocket for ConnectProxy {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.conn.read(buf)
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.conn.write(buf)
    }

    fn close(&mut self) -> io::Result<()> {
        self.conn.shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }

    fn get_addr(&self) -> &[u8] {
        &self.addr_buf
    }
}

fn new_connect_proxy(addr: impl ToSocketAddrs) -> io::Result<ConnectProxy> {
    let mut buf = String::new();
    let mut conn = TcpStream::connect(addr)?;

    // 1. receive CONNECT request..
    let mut rd = io::BufReader::new(conn.try_clone()?);
    rd.read_line(&mut buf)?;
    let words: Vec<&str> = buf.trim().split(' ').collect();
    if words.len() < 2 || words[0] != "CONNECT" {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("invalid CONNECT request: {}", buf),
        ));
    }

    conn.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;

    let chost = words[1];
    // 3. sends a HEADERS frame containing a 2xx series status code to the client, as defined in [RFC7231], Section 4.3.6
    let res1: Vec<&str> = chost.split(':').collect();
    let hostname = res1[0];
    let port = res1[1];

    let addr_buf = toolbox::build_socks5_addr_data(hostname, port)?;
    Ok(ConnectProxy { addr_buf, conn })
}
