use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

fn main() {
    let addr = "127.0.0.1:1090".to_socket_addrs().unwrap().next().unwrap();
    let listener = TcpListener::bind(addr).unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || process(stream));
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

fn process(mut client: TcpStream) {
    if let Err(e) = socks5_auth(&mut client) {
        eprintln!("Auth error: {}", e);
        return;
    }

    let mut dst = match socks5_connect(&mut client) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Connect error: {}", e);
            return;
        }
    };

    socks5_forward(&mut client, &mut dst);
}

fn socks5_auth(client: &mut TcpStream) -> Result<(), String> {
    let mut buf = [0; 256];

    // 读取 VER 和 NMETHODS
    client.read_exact(&mut buf[0..2]).map_err(|e| format!("Reading header: {}", e))?;

    let ver = buf[0] as usize;
    let nmethods = buf[1] as usize;
    if ver != 5 {
        return Err("Invalid version".to_owned());
    }

    // 读取 METHODS 列表
    client.read_exact(&mut buf[0..nmethods]).map_err(|e| format!("Reading methods: {}", e))?;

    //无需认证
    client.write_all(&[0x05, 0x00]).map_err(|e| format!("Write response: {}", e))?;
    Ok(())
}

fn socks5_connect(client: &mut TcpStream) -> Result<TcpStream, String> {
    let mut buf = [0; 256];

    client.read_exact(&mut buf[0..4]).map_err(|e| format!("Reading header: {}", e))?;

    let ver = buf[0] as usize;
    let cmd = buf[1] as usize;
    let _ = buf[2];
    let atyp = buf[3] as usize;

    if ver != 5 || cmd != 1 {
        return Err("Invalid ver/cmd".to_owned());
    }

    let addr = match atyp {
        1 => {
            client.read_exact(&mut buf[0..4]).map_err(|e| format!("Invalid IPv4: {}", e))?;
            format!("{}.{}.{}.{}", buf[0], buf[1], buf[2], buf[3])
        }
        3 => {
            client.read_exact(&mut buf[0..1]).map_err(|e| format!("Invalid hostname: {}", e))?;
            let len = buf[0] as usize;
            client.read_exact(&mut buf[0..len]).map_err(|e| format!("Invalid hostname: {}", e))?;
            String::from_utf8_lossy(&buf[0..len]).to_string()
        }
        4 => return Err("IPv6: not supported yet".to_owned()),
        _ => return Err("Invalid atyp".to_owned()),
    };

    client.read_exact(&mut buf[0..2]).map_err(|e| format!("Read port: {}", e))?;
    let port = u16::from_be_bytes([buf[0], buf[1]]);
    let target = format!("{}:{}", addr, port);
    println!("connecting {}",target);
    let dst = TcpStream::connect(target).map_err(|e| format!("Dial dst: {}", e))?;

    client.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).map_err(|e| format!("Write response: {}", e))?;

    Ok(dst)
}

fn socks5_forward(client: &mut TcpStream, target: &mut TcpStream) {
    let f1 = std::thread::spawn({
        let mut client = client.try_clone().unwrap();
        let mut target = target.try_clone().unwrap();
        move || {
            match std::io::copy(&mut client, &mut target) {
                Ok(_) => (),
                Err(e) => eprintln!("Error forwarding from client to target: {}", e),
            };
        }
    });

    let f2 = std::thread::spawn({
        let mut target = target.try_clone().unwrap();
        let mut client = client.try_clone().unwrap();
        move || {
            match std::io::copy(&mut target, &mut client) {
                Ok(_) => (),
                Err(e) => eprintln!("Error forwarding from target to client: {}", e),
            };
        }
    });

    f1.join().unwrap();
    f2.join().unwrap();
}
