use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::Inbound;
use crate::utils::socks5::build_socks5_buffer;

fn io_other<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e.to_string())
}

/// Parses an HTTP CONNECT request and replies 200, then returns the SOCKS5
/// encoded target address for tunneling (matching Go's connect proxy).
pub async fn handshake(mut conn: TcpStream) -> io::Result<Inbound> {
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        conn.read_exact(&mut byte).await?;
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
        if buf.len() > 8192 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "request header too large"));
        }
    }

    let text = String::from_utf8_lossy(&buf);
    let first = text.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split_whitespace().collect();
    if parts.len() < 2 || parts[0] != "CONNECT" {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "CONNECT token mismatch"));
    }
    let hostport = parts[1];
    let (hostname, port) = hostport
        .rsplit_once(':')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid address"))?;
    let port: u16 = port
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid port"))?;

    conn.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;

    let addr = build_socks5_buffer(hostname, port).map_err(io_other)?;
    Ok(Inbound::Tcp(conn, addr))
}
