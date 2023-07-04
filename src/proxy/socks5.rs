use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;

use super::base::ProxySocket;

pub fn new_socks5_proxy(mut conn: TcpStream) -> Result<ProxySocket, Box<dyn Error>> {
    let mut buf = [0; 256];

    // 读取 VER 和 NMETHODS
    conn.read_exact(&mut buf[0..2])?;

    let ver = buf[0] as usize;
    let nmethods = buf[1] as usize;
    if ver != 5 {
        return Err("socks5 ver invalid!".into());
    }

    // 读取 METHODS 列表
    conn.read_exact(&mut buf[0..nmethods])?;

    //无需认证
    conn.write_all(&[0x05, 0x00])?;

    // read COMMAND
    let (ver, cmd, _, atyp) = {
        conn.read_exact(&mut buf[0..4])?;
        (buf[0], buf[1], buf[2], buf[3])
    };

    buf[0] = atyp;

    let addrlen: usize = match atyp {
        0x01 => {
            conn.read_exact(&mut buf[1..7])?;
            1 + 4 + 2 //atype+ host + port
        }
        0x03 => {
            // domain name
            let domain_len = read_byte(&mut conn)?;
            let len = (2 + domain_len) as usize;
            buf[1] = domain_len;
            conn.read_exact(&mut buf[2..len + 2])?;
            1 + 1 + domain_len as usize + 2 // atype+domain_len+domain name + port + NAMETYPE
        }
        _ => return Err("invalid ATYP".into()),
    };

    let add_buf = &buf[..addrlen];
    conn.write_all(&[0x05, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?;

    let s = ProxySocket::new(add_buf.to_vec(), conn);
    Ok(s)
}

fn read_byte(conn: &mut TcpStream) -> Result<u8, Box<dyn Error>> {
    let mut buf = [0u8; 1];
    conn.read_exact(&mut buf)?;
    Ok(buf[0])
}
