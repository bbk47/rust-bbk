use bbk::proxy::udprelay::{is_udp_marker, read_record, udp_marker, write_record};

#[test]
fn udp_marker_roundtrip() {
    let m = udp_marker();
    assert_eq!(m, vec![0xFD, b'U', b'D', b'P']);
    assert!(is_udp_marker(&m));
    assert!(!is_udp_marker(b"\x01\x7f\x00\x00\x01\x00\x50")); // a normal ipv4 socks5 addr
    assert!(!is_udp_marker(&[]));
}

/// `[u16 len][payload]` framing must round-trip, including back-to-back records
/// and an empty payload, exactly as the Go UDP relay frames datagrams.
#[tokio::test]
async fn udp_record_framing_roundtrip() {
    let (mut a, mut b) = tokio::io::duplex(64 * 1024);

    let payloads: Vec<Vec<u8>> = vec![b"first".to_vec(), Vec::new(), vec![0xABu8; 1500], b"last".to_vec()];
    let writer_payloads = payloads.clone();

    let writer = tokio::spawn(async move {
        for p in &writer_payloads {
            write_record(&mut a, p).await.unwrap();
        }
        a // keep alive until all reads done
    });

    for expected in &payloads {
        let got = read_record(&mut b).await.unwrap();
        assert_eq!(&got, expected);
    }
    let _ = writer.await.unwrap();
}

#[tokio::test]
async fn udp_record_rejects_oversized() {
    let (mut a, _b) = tokio::io::duplex(1024);
    let too_big = vec![0u8; 64 * 1024 + 1];
    assert!(write_record(&mut a, &too_big).await.is_err());
}
