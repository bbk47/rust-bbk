#[path = "../src/utils/socks5.rs"]
mod socks5;

use socks5::build_socks5_address_data;
use socks5::AddrInfo;
use std::net::Ipv6Addr;

#[test]
fn parse_addr_info_ipv4() {
    let buffer = [1, 127, 0, 0, 1, 0x80, 0x80];
    let address = AddrInfo::from_buffer(&buffer).unwrap();
    assert_eq!(
        address,
        AddrInfo {
            address: "127.0.0.1".to_owned(),
            port: 32896,
        }
    );
}

#[test]
fn parse_addr_info_ipv6() {
    let buffer = [4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 127, 0, 0, 1, 0, 1];
    let addrinfo = AddrInfo::from_buffer(&buffer).unwrap();

    let ipv6_addr1 = addrinfo.address.parse::<Ipv6Addr>().unwrap();
    let ipv6_addr2 = "::ffff:7f00:1".parse::<Ipv6Addr>().unwrap();
    assert_eq!(ipv6_addr1, ipv6_addr2);
    assert_eq!(addrinfo.port, 1);
}

#[test]
fn parse_addr_info_domain() {
    let buffer = [3, 11, 101, 120, 97, 109, 112, 108, 101, 46, 99, 111, 109, 0x80, 0x80];
    // [101, 120, 97, 109, 112, 108, 101, 46, 99, 111, 109]
    let address = AddrInfo::from_buffer(&buffer).unwrap();
    assert_eq!(
        address,
        AddrInfo {
            address: "example.com".to_owned(),
            port: 32896,
        }
    );
}

#[test]
fn fail_parse_invalid_addr_type() {
    let buffer = [5, 0, 0];
    let address = AddrInfo::from_buffer(&buffer);
    assert!(address.is_err());
    assert_eq!(address.unwrap_err().to_string(), "invalid address type");
}

#[test]
fn build_socks5_address_data_ipv4() {
    let buffer = build_socks5_address_data("127.0.0.1", 32896).unwrap();
    assert_eq!(buffer, [1, 127, 0, 0, 1, 0x80, 0x80]);
}

#[test]
fn build_socks5_address_data_ipv6() {
    let buffer = build_socks5_address_data("::ffff:7f00:1", 1).unwrap();
    assert_eq!(buffer, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 127, 0, 0, 1, 0, 1,]);
}

#[test]
fn build_socks5_address_data_domain() {
    let buffer = build_socks5_address_data("www.example.com", 32896).unwrap();
    assert_eq!(buffer, [3, 15, 119, 119, 119, 46, 101, 120, 97, 109, 112, 108, 101, 46, 99, 111, 109, 0x80, 0x80,]);
}
