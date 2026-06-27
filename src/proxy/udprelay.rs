use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use crate::tunnel::Stream;
use crate::utils::socks5::{socks5_addr_len, AddrInfo};

const UDP_MARKER: [u8; 4] = [0xFD, b'U', b'D', b'P'];
const MAX_UDP_DATAGRAM: usize = 64 * 1024;
const UDP_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Sentinel "address" used to flag a stream as a UDP association.
pub fn udp_marker() -> Vec<u8> {
    UDP_MARKER.to_vec()
}

pub fn is_udp_marker(addr: &[u8]) -> bool {
    addr == UDP_MARKER
}

pub async fn write_record<W: AsyncWrite + Unpin>(w: &mut W, payload: &[u8]) -> io::Result<()> {
    if payload.len() > MAX_UDP_DATAGRAM {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "udp datagram too large"));
    }
    let mut buf = Vec::with_capacity(2 + payload.len());
    buf.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    buf.extend_from_slice(payload);
    w.write_all(&buf).await
}

pub async fn read_record<R: AsyncRead + Unpin>(r: &mut R) -> io::Result<Vec<u8>> {
    let mut lb = [0u8; 2];
    r.read_exact(&mut lb).await?;
    let n = u16::from_be_bytes(lb) as usize;
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Client side: bridge the local SOCKS5 UDP socket and the tunnel stream.
/// App datagrams are `[RSV RSV FRAG][socks5addr][data]`; the first 3 bytes are
/// stripped before tunneling, and re-prepended on the way back.
pub async fn client_udp(udp: UdpSocket, stream: Stream) {
    let udp = Arc::new(udp);
    let (mut sr, mut sw) = tokio::io::split(stream);
    let peer: Arc<Mutex<Option<SocketAddr>>> = Arc::new(Mutex::new(None));

    let udp_up = udp.clone();
    let peer_up = peer.clone();
    let up = async move {
        let mut buf = vec![0u8; MAX_UDP_DATAGRAM];
        loop {
            let (n, src) = match udp_up.recv_from(&mut buf).await {
                Ok(v) => v,
                Err(_) => break,
            };
            *peer_up.lock().await = Some(src);
            if n < 3 || buf[2] != 0x00 {
                continue; // need FRAG == 0
            }
            if write_record(&mut sw, &buf[3..n]).await.is_err() {
                break;
            }
        }
    };

    let down = async move {
        loop {
            let record = match read_record(&mut sr).await {
                Ok(r) => r,
                Err(_) => break,
            };
            let dst = { *peer.lock().await };
            if let Some(dst) = dst {
                let mut out = Vec::with_capacity(3 + record.len());
                out.extend_from_slice(&[0, 0, 0]);
                out.extend_from_slice(&record);
                if udp.send_to(&out, dst).await.is_err() {
                    break;
                }
            }
        }
    };

    tokio::select! {
        _ = up => {}
        _ = down => {}
    }
}

/// Server side: read `[socks5addr][data]` records from the tunnel stream, relay
/// to UDP targets, and stream replies back tagged with the same address. A
/// per-target socket is kept with a 60s idle timeout.
pub async fn serve_udp(stream: Stream) {
    let (mut sr, sw) = tokio::io::split(stream);
    let sw = Arc::new(Mutex::new(sw));
    let mut sessions: HashMap<String, Arc<UdpSocket>> = HashMap::new();

    loop {
        let record = match read_record(&mut sr).await {
            Ok(r) => r,
            Err(_) => break,
        };
        let alen = match socks5_addr_len(&record) {
            Some(l) if l <= record.len() => l,
            _ => continue,
        };
        // Parse only the address bytes; the record also carries the payload,
        // and AddrInfo derives the port from the end of the slice it is given.
        let info = match AddrInfo::from_buffer(&record[..alen]) {
            Ok(i) => i,
            Err(_) => continue,
        };
        let addr_bytes = record[..alen].to_vec();
        let payload = &record[alen..];
        let key = format!("{}:{}", info.host, info.port);

        let sock = match sessions.get(&key) {
            Some(s) => s.clone(),
            None => {
                let s = match UdpSocket::bind("0.0.0.0:0").await {
                    Ok(s) => Arc::new(s),
                    Err(_) => continue,
                };
                if s.connect((info.host.as_str(), info.port)).await.is_err() {
                    continue;
                }
                let sw_reply = sw.clone();
                let s_reply = s.clone();
                let prefix = addr_bytes.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; MAX_UDP_DATAGRAM];
                    loop {
                        let n = match tokio::time::timeout(UDP_IDLE_TIMEOUT, s_reply.recv(&mut buf)).await {
                            Ok(Ok(n)) => n,
                            _ => break,
                        };
                        let mut rec = prefix.clone();
                        rec.extend_from_slice(&buf[..n]);
                        let mut w = sw_reply.lock().await;
                        if write_record(&mut *w, &rec).await.is_err() {
                            break;
                        }
                    }
                });
                sessions.insert(key.clone(), s.clone());
                s
            }
        };
        let _ = sock.send(payload).await;
    }
}
