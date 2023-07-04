use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::println;
use std::str::FromStr;

#[derive(Debug)]
struct InvalidAddressType;

impl Display for InvalidAddressType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "invalid address type")
    }
}

impl Error for InvalidAddressType {}

#[derive(Debug, PartialEq)]
pub struct AddrInfo {
    pub host: String,
    pub port: u16,
}

impl AddrInfo {
    pub fn from_buffer(buffer: &[u8]) -> Result<Self, Box<dyn Error>> {
        match buffer.first() {
            Some(&1) => {
                let mut ip = [0; 4];
                ip.copy_from_slice(&buffer[1..=4]);
                let host = Ipv4Addr::from(ip).to_string();
                let port = u16::from_be_bytes([buffer[buffer.len() - 2], buffer.last().copied().unwrap_or(0x00)]);
                Ok(AddrInfo { host, port })
            }

            Some(&3) => {
                let domain_length: usize = buffer[1] as usize;
                // println!("domain_length:{}",&domain_length);
                let domain_end = 2 + domain_length;
                let host: String = String::from_utf8_lossy(&buffer[2..domain_end]).to_string();
                let port = u16::from_be_bytes([buffer[buffer.len() - 2], buffer.last().copied().unwrap_or(0x00)]);
                // println!("parse socks5 addr:{},{}",host,port);
                Ok(AddrInfo { host, port })
            }

            Some(&4) => {
                let mut ip = [0; 16];
                ip.copy_from_slice(&buffer[1..=16]);
                let host = Ipv6Addr::from(ip).to_string();
                let port = u16::from_be_bytes([buffer[buffer.len() - 2], buffer.last().copied().unwrap_or(0x00)]);
                Ok(AddrInfo { host, port })
            }

            _ => Err(Box::new(InvalidAddressType)),
        }
    }
}

pub fn build_socks5_buffer(host: &str, port: u16) -> Result<Vec<u8>, Box<dyn Error>> {
    if let Ok(ipv4_host) = Ipv4Addr::from_str(host) {
        Ok(vec![1].into_iter().chain(ipv4_host.octets().iter().copied()).chain(port.to_be_bytes().iter().copied()).collect())
    } else if let Ok(ipv6_host) = Ipv6Addr::from_str(host) {
        Ok(vec![4].into_iter().chain(ipv6_host.octets().iter().copied()).chain(port.to_be_bytes().iter().copied()).collect())
    } else {
        let domain_length = host.len();
        let mut buffer = vec![3, domain_length as u8];
        buffer.extend_from_slice(host.as_bytes());
        buffer.extend_from_slice(&port.to_be_bytes());
        Ok(buffer)
    }
}
