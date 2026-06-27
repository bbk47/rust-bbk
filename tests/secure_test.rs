use bbk::tunnel::SecureConn;
use bbk::utils::encrypt::Encryptor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Drive the random-IV handshake on both ends over an in-memory pipe, then send
/// data in both directions. This exercises the IV exchange plus the continuous
/// stream cipher wired through poll_read/poll_write.
async fn secure_pair(method: &str) -> (SecureConn<tokio::io::DuplexStream>, SecureConn<tokio::io::DuplexStream>) {
    let (c_io, s_io) = tokio::io::duplex(64 * 1024);
    let enc_c = Encryptor::new(method, "p@ssword").unwrap();
    let enc_s = Encryptor::new(method, "p@ssword").unwrap();
    let (c, s) = tokio::join!(SecureConn::handshake(c_io, &enc_c), SecureConn::handshake(s_io, &enc_s),);
    (c.unwrap(), s.unwrap())
}

#[tokio::test]
async fn secure_conn_bidirectional() {
    for method in ["aes-256-cfb", "aes-128-ctr", "rc4-md5"] {
        let (mut client, mut server) = secure_pair(method).await;

        // client -> server
        client.write_all(b"ping over the tunnel").await.unwrap();
        client.flush().await.unwrap();
        let mut buf = [0u8; 20];
        server.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"ping over the tunnel", "method={method}");

        // server -> client
        server.write_all(b"pong back").await.unwrap();
        server.flush().await.unwrap();
        let mut buf2 = [0u8; 9];
        client.read_exact(&mut buf2).await.unwrap();
        assert_eq!(&buf2, b"pong back", "method={method}");
    }
}

/// A large, chunked transfer must reassemble correctly, proving the keystream
/// stays in sync across many writes/reads.
#[tokio::test]
async fn secure_conn_large_stream() {
    let (mut client, mut server) = secure_pair("aes-256-ctr").await;

    let total = 256 * 1024usize;
    let writer = tokio::spawn(async move {
        let mut sent = 0usize;
        while sent < total {
            let n = (total - sent).min(4096);
            // Byte at global offset o carries value (o % 251) so the reader can
            // verify ordering across chunk boundaries.
            let chunk: Vec<u8> = (0..n).map(|k| ((sent + k) % 251) as u8).collect();
            client.write_all(&chunk).await.unwrap();
            sent += n;
        }
        client.flush().await.unwrap();
        client
    });

    let mut received = 0usize;
    let mut buf = vec![0u8; 8192];
    while received < total {
        let n = server.read(&mut buf).await.unwrap();
        assert!(n > 0, "unexpected EOF at {received}");
        for &byte in &buf[..n] {
            assert_eq!(byte, (received % 251) as u8, "mismatch at offset {received}");
            received += 1;
        }
    }
    let _ = writer.await.unwrap();
}
