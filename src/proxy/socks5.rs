use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

use super::{Inbound, Socks5UdpProxy};
use crate::utils::socks5::build_socks5_buffer;

const CMD_CONNECT: u8 = 0x01;
const CMD_UDP_ASSOCIATE: u8 = 0x03;

fn io_other<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e.to_string())
}

/// Performs the SOCKS5 negotiation + request phase. Supports CONNECT and UDP
/// ASSOCIATE (matching Go's socks5 proxy).
pub async fn handshake(mut conn: TcpStream) -> io::Result<Inbound> {
    // Method negotiation.
    let mut head = [0u8; 2];
    conn.read_exact(&mut head).await?;
    if head[0] != 0x05 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "socks5 version invalid"));
    }
    let nmethods = head[1] as usize;
    let mut methods = vec![0u8; nmethods];
    conn.read_exact(&mut methods).await?;
    conn.write_all(&[0x05, 0x00]).await?; // no auth

    // Request: VER CMD RSV ATYP.
    let mut req = [0u8; 4];
    conn.read_exact(&mut req).await?;
    let cmd = req[1];
    let atyp = req[3];

    let mut addr = vec![atyp];
    match atyp {
        0x01 => {
            let mut b = [0u8; 6];
            conn.read_exact(&mut b).await?;
            addr.extend_from_slice(&b);
        }
        0x03 => {
            let mut l = [0u8; 1];
            conn.read_exact(&mut l).await?;
            addr.push(l[0]);
            let mut b = vec![0u8; l[0] as usize + 2];
            conn.read_exact(&mut b).await?;
            addr.extend_from_slice(&b);
        }
        0x04 => {
            let mut b = [0u8; 18];
            conn.read_exact(&mut b).await?;
            addr.extend_from_slice(&b);
        }
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid ATYP")),
    }

    match cmd {
        CMD_CONNECT => {
            conn.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
            Ok(Inbound::Tcp(conn, addr))
        }
        CMD_UDP_ASSOCIATE => {
            let local_ip = conn.local_addr()?.ip();
            let udp = UdpSocket::bind((local_ip, 0)).await?;
            let bnd = udp.local_addr()?;
            let mut reply = vec![0x05, 0x00, 0x00];
            reply.extend_from_slice(&build_socks5_buffer(&bnd.ip().to_string(), bnd.port()).map_err(io_other)?);
            conn.write_all(&reply).await?;
            Ok(Inbound::Udp(Socks5UdpProxy { ctrl: conn, udp }))
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported socks5 command")),
    }
}
