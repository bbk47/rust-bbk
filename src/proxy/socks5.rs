use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};


trait ProxySocket {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
    fn close(&mut self) -> std::io::Result<()>;
    fn get_addr(&self) -> &[u8];
}

struct Socks5Proxy {
    addr_buf: Vec<u8>,
    conn: TcpStream,
}

impl ProxySocket for Socks5Proxy {
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

fn new_socks5_proxy(addr: impl ToSocketAddrs) -> io::Result<Socks5Proxy> {
    let mut buf = vec![0; 256];
    let mut conn = TcpStream::connect(addr)?;

    // read VER and NMETHODS
    let n = conn.read(&mut buf[..2])?;
    if n != 2 {
        return Err(io::Error::new(io::ErrorKind::Other, "socks5 ver/method read failed!"));
    }

    let ver = buf[0] as usize;
    let n_methods = buf[1] as usize;
    if ver != 5 {
        return Err(io::Error::new(io::ErrorKind::Other, "socks5 ver invalid!"));
    }

    // read METHODS list
    let n = conn.read(&mut buf[..n_methods])?;
    if n != n_methods {
        return Err(io::Error::new(io::ErrorKind::Other, "socks5 method err!"));
    }

    // INIT
    // no authentication required
    conn.write_all(&[0x05, 0x00])?;

    n = io::Read::read(&mut conn, &mut buf[..4])?;
    if n != 4 {
        return Err(io::Error::new(io::ErrorKind::Other, format!("protol error: {}", err)));
    }

    let ver = buf[0] as usize;
    let cmd = buf[1];
    let _rsv = buf[2];
    let atyp = buf[3];

    if ver != 5 || cmd != 1 {
        return Err(io::Error::new(io::ErrorKind::Other, "invalid ver/cmd"));
    }

    let (addr_buf, addr_len) = match atyp {
        0x01 => {
            io::Read::read_exact(&mut conn, &mut buf[4..10])?;
            (&buf[3..10], 7)
        }
        0x03 => {
            let n = io::Read::read(&mut conn, &mut buf[4..5])?;
            let domain_len = buf[4] as usize;
            io::Read::read_exact(&mut conn, &mut buf[5..(domain_len + 5 + 2)])?;
            (&buf[3..(domain_len + 5 + 2)], domain_len + 4)
        }
        _ => {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsupported address type"));
        }
    };

    // COMMAND RESP
    conn.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])?;

    Ok(Socks5Proxy {
        addr_buf: addr_buf.to_vec(),
        conn,
    })
}
