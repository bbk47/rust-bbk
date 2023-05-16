use std::error::Error;
use std::io::{self, Read, Write};
use std::net::TcpStream;

pub trait ProxySocket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn close(&mut self) -> io::Result<()>;
    fn get_addr(&self) -> &[u8];
}

pub struct Socks5Proxy {
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
        self.conn.shutdown(std::net::Shutdown::Both)
    }

    fn get_addr(&self) -> &[u8] {
        &self.addr_buf[..]
    }
}

pub fn new_socks5_proxy(mut conn: TcpStream) -> Result<Socks5Proxy, Box<dyn Error>> {
    let mut buf = [0u8; 256];

    // read VER and NMETHODS
    let (ver, n_methods) = {
        conn.read_exact(&mut buf[0..2])?;
        let n_methods = buf[1] as usize;
        (buf[0], n_methods + 2) // +2 bytes for VER and NMETHODS
    };

    if ver != 0x05 {
        return Err("socks5 ver invalid!".into());
    }

    // read METHODS list
    conn.read_exact(&mut buf[0..n_methods])?;

    // INIT
    // no authentication required
    conn.write_all(&[0x05u8, 0x00u8])?;

    // read COMMAND
    let (ver, cmd, _, atyp) = {
        conn.read_exact(&mut buf[0..4])?;
        (buf[0], buf[1], buf[2], buf[3])
    };

    if ver != 0x05 || cmd != 0x01 {
        return Err("invalid ver/cmd".into());
    }

    let addr_len = match atyp {
        0x01 => {
            conn.read_exact(&mut buf[0..6])?;
            4 + 2 // host + port
        }
        0x03 => {
            // domain name
            let domain_len = read_byte(&mut conn)?;
            conn.read_exact(&mut buf[0..(domain_len + 2)])?;
            domain_len as usize + 2 + 1 // domain name + port + NAMETYPE
        }
        _ => return Err("invalid ATYP".into()),
    };

    let add_buf = &buf[3..3 + addr_len];
    conn.write_all(&[0x05u8, 0x00u8, 0x00u8, 0x01u8, 0x00u8, 0x00u8, 0x00u8, 0x00u8, 0x00u8, 0x00u8])?;

    let s = Socks5Proxy { addr_buf: add_buf.to_vec(), conn };
    Ok(s)
}

fn read_byte(conn: &mut TcpStream) -> Result<u8, Box<dyn Error>> {
    let mut buf = [0u8; 1];
    conn.read_exact(&mut buf)?;
    Ok(buf[0])
}
