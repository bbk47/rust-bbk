use std::time::Duration;

use bbk::tunnel::{client_session, server_session, Session};
use bbk::utils::encrypt::Encryptor;
use bbk::utils::socks5::build_socks5_buffer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Spawn a yamux server session over an in-memory pipe that echoes every stream
/// back to the client (after the `set_ready` status handshake), and return a
/// connected client session. This mirrors the real client<->server data path
/// minus the OS sockets / transport.
async fn echo_tunnel(method: &str) -> Session {
    let (c_io, s_io) = tokio::io::duplex(256 * 1024);
    let enc_c = Encryptor::new(method, "p@ssword").unwrap();
    let enc_s = Encryptor::new(method, "p@ssword").unwrap();

    tokio::spawn(async move {
        let sess = match server_session(s_io, &enc_s).await {
            Ok(s) => s,
            Err(_) => return,
        };
        loop {
            let mut stream = match sess.accept_stream().await {
                Ok(s) => s,
                Err(_) => break,
            };
            tokio::spawn(async move {
                if stream.set_ready().await.is_err() {
                    return;
                }
                let mut buf = vec![0u8; 8192];
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if stream.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                            let _ = stream.flush().await;
                        }
                    }
                }
            });
        }
    });

    client_session(c_io, &enc_c).await.expect("client session")
}

#[tokio::test]
async fn tunnel_open_stream_and_echo() {
    let test = async {
        let sess = echo_tunnel("aes-256-cfb").await;
        let addr = build_socks5_buffer("93.184.216.34", 80).unwrap();
        let mut stream = sess.open_stream(&addr).await.unwrap();
        // The address handshake must preserve the target bytes verbatim.
        assert_eq!(stream.addr, addr);

        stream.write_all(b"hello tunnel").await.unwrap();
        stream.flush().await.unwrap();
        let mut buf = [0u8; 12];
        stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hello tunnel");
    };
    tokio::time::timeout(Duration::from_secs(10), test).await.expect("tunnel test timed out");
}

/// yamux must multiplex several concurrent streams over the one connection.
#[tokio::test]
async fn tunnel_multiplexes_concurrent_streams() {
    let test = async {
        let sess = echo_tunnel("aes-128-ctr").await;
        let mut handles = Vec::new();
        for i in 0..8u8 {
            let addr = build_socks5_buffer("10.0.0.1", 1000 + i as u16).unwrap();
            let mut stream = sess.open_stream(&addr).await.unwrap();
            handles.push(tokio::spawn(async move {
                let msg = vec![i; 4096];
                stream.write_all(&msg).await.unwrap();
                stream.flush().await.unwrap();
                let mut got = vec![0u8; 4096];
                stream.read_exact(&mut got).await.unwrap();
                assert_eq!(got, msg, "stream {i} echo mismatch");
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
    };
    tokio::time::timeout(Duration::from_secs(15), test).await.expect("mux test timed out");
}

#[tokio::test]
async fn tunnel_refused_when_server_drops_stream() {
    // Server that accepts a stream but never sets it ready, then drops it:
    // open_stream should surface an error rather than hang.
    let (c_io, s_io) = tokio::io::duplex(64 * 1024);
    let enc_c = Encryptor::new("aes-256-cfb", "p@ssword").unwrap();
    let enc_s = Encryptor::new("aes-256-cfb", "p@ssword").unwrap();
    tokio::spawn(async move {
        if let Ok(sess) = server_session(s_io, &enc_s).await {
            if let Ok(stream) = sess.accept_stream().await {
                drop(stream); // FIN without status byte
            }
            // keep the session alive briefly so the FIN propagates
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });

    let sess = client_session(c_io, &enc_c).await.unwrap();
    let addr = build_socks5_buffer("1.2.3.4", 53).unwrap();
    let res = tokio::time::timeout(Duration::from_secs(5), sess.open_stream(&addr)).await;
    match res {
        Ok(r) => assert!(r.is_err(), "expected refusal, got ok"),
        Err(_) => panic!("open_stream hung instead of failing"),
    }
}
