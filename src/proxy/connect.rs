use std::error::Error;
use std::io::BufRead;
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;

use super::base::ProxySocket;
use crate::utils;

pub fn new_connect_proxy(mut conn: TcpStream) -> Result<ProxySocket, Box<dyn Error>> {
    let mut buf_reader: BufReader<&mut TcpStream> = BufReader::new(&mut conn);
    let mut buf: Vec<u8> = Vec::new();

    // read CONNECT request
    let mut line_buf = String::new();
    buf_reader.read_line(&mut line_buf)?;
    let words: Vec<&str> = line_buf.split_whitespace().collect();
    if words.len() < 2 || words[0] != "CONNECT" {
        return Err("CONNECT token mismatch!".into());
    }
    let chost = words[1];

    // sends a OK response
    conn.write_all(b"HTTP/1.1 200 OK\r\n\r\n")?;

    // parse host and port
    let (hostname, port) = {
        let parts: Vec<&str> = chost.split(':').collect();
        if parts.len() != 2 {
            return Err("invalid address".into());
        }
        (parts[0], parts[1])
    };

    let port: u16 = port.parse().unwrap();
    // build socks5 address data
    let addr_data = utils::socks5::build_socks5_address_data(hostname, port)?;
    let s = ProxySocket::new(addr_data, conn);
    Ok(s)
}
