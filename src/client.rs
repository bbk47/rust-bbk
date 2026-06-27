use std::io;
use std::sync::Arc;
use std::time::Duration;

use log::{error, info};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::option::{BbkCliOption, TunnelOpts};
use crate::proxy::{self, Inbound};
use crate::transport;
use crate::tunnel::{self, Session, WsConn};
use crate::utils::encrypt::Encryptor;
use crate::utils::relay;

pub struct BbkClient {
    opts: BbkCliOption,
    tunnel_opts: TunnelOpts,
    enc: Arc<Encryptor>,
    session: Mutex<Option<Arc<Session>>>,
}

impl BbkClient {
    pub fn new(opts: BbkCliOption) -> Self {
        let tunnel_opts = opts.tunnel_opts.clone().expect("client requires tunnelOpts");
        let enc = Arc::new(Encryptor::new(&tunnel_opts.method, &tunnel_opts.password).expect("invalid encryption method"));
        BbkClient {
            opts,
            tunnel_opts,
            enc,
            session: Mutex::new(None),
        }
    }

    pub async fn bootstrap(self) {
        let client = Arc::new(self);

        // Pre-warm the tunnel (non-fatal on failure).
        if let Err(e) = client.get_session().await {
            error!("bootstrap tunnel failed: {}", e);
        }

        let mut handles = Vec::new();
        let host = client.opts.listen_addr.clone();

        let socks_port = client.opts.listen_port;
        if socks_port > 1024 {
            let c = client.clone();
            let host = host.clone();
            handles.push(tokio::spawn(async move {
                let handler = move |sock| {
                    let c = c.clone();
                    async move { c.handle_socks5(sock).await }
                };
                info!("socks5 proxy listen on {}:{}", host, socks_port);
                if let Err(e) = proxy::listen(&host, socks_port as u16, handler).await {
                    error!("socks5 listen error: {}", e);
                }
            }));
        }

        let http_port = client.opts.listen_http_port;
        if http_port > 1080 {
            let c = client.clone();
            let host = host.clone();
            handles.push(tokio::spawn(async move {
                let handler = move |sock| {
                    let c = c.clone();
                    async move { c.handle_connect(sock).await }
                };
                info!("http connect proxy listen on {}:{}", host, http_port);
                if let Err(e) = proxy::listen(&host, http_port as u16, handler).await {
                    error!("connect listen error: {}", e);
                }
            }));
        }

        for h in handles {
            let _ = h.await;
        }
    }

    async fn setup_tunnel(&self) -> io::Result<Session> {
        let t = &self.tunnel_opts;
        match t.protocol.as_str() {
            "tcp" => {
                let c = transport::dial_tcp(&t.host, &t.port).await?;
                tunnel::client_session(c, &self.enc).await
            }
            "tls" => {
                let c = transport::dial_tls(&t.host, &t.port).await?;
                tunnel::client_session(c, &self.enc).await
            }
            "ws" => {
                let c = transport::dial_ws(&t.host, &t.port, &t.path, t.secure).await?;
                tunnel::client_session(WsConn::new(c), &self.enc).await
            }
            "h2" => {
                let c = transport::h2conn::dial_h2(&t.host, &t.port, &t.path).await?;
                tunnel::client_session(c, &self.enc).await
            }
            other => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("unknown protocol: {}", other))),
        }
    }

    /// Returns a usable session, rebuilding it if absent or closed. yamux
    /// keepalive marks dead links closed, triggering reconnect here.
    async fn get_session(&self) -> io::Result<Arc<Session>> {
        let mut guard = self.session.lock().await;
        if let Some(s) = guard.as_ref() {
            if !s.is_closed() {
                return Ok(s.clone());
            }
        }
        let mut last_err: Option<io::Error> = None;
        for attempt in 0..5 {
            match self.setup_tunnel().await {
                Ok(s) => {
                    let s = Arc::new(s);
                    *guard = Some(s.clone());
                    info!("tunnel connected via {}", self.tunnel_opts.protocol);
                    return Ok(s);
                }
                Err(e) => {
                    error!("tunnel setup attempt {} failed: {}", attempt + 1, e);
                    last_err = Some(e);
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        }
        Err(last_err.unwrap_or_else(|| io::Error::new(io::ErrorKind::Other, "tunnel setup failed")))
    }

    async fn handle_socks5(self: Arc<Self>, sock: TcpStream) {
        match proxy::socks5::handshake(sock).await {
            Ok(inbound) => self.bind_inbound(inbound).await,
            Err(e) => error!("socks5 handshake: {}", e),
        }
    }

    async fn handle_connect(self: Arc<Self>, sock: TcpStream) {
        match proxy::connect::handshake(sock).await {
            Ok(inbound) => self.bind_inbound(inbound).await,
            Err(e) => error!("connect handshake: {}", e),
        }
    }

    async fn bind_inbound(&self, inbound: Inbound) {
        let sess = match self.get_session().await {
            Ok(s) => s,
            Err(e) => {
                error!("get session: {}", e);
                return;
            }
        };
        match inbound {
            Inbound::Tcp(mut sock, addr) => {
                let mut stream = match sess.open_stream(&addr).await {
                    Ok(s) => s,
                    Err(e) => {
                        error!("open stream: {}", e);
                        return;
                    }
                };
                relay(&mut sock, &mut stream).await;
            }
            Inbound::Udp(udp_proxy) => {
                let stream = match sess.open_stream(&proxy::udp_marker()).await {
                    Ok(s) => s,
                    Err(e) => {
                        error!("open udp stream: {}", e);
                        return;
                    }
                };
                let mut ctrl = udp_proxy.ctrl;
                let udp = udp_proxy.udp;
                // SOCKS5: closing the control TCP ends the UDP association.
                tokio::select! {
                    _ = proxy::client_udp(udp, stream) => {}
                    _ = drain(&mut ctrl) => {}
                }
            }
        }
    }
}

async fn drain(conn: &mut TcpStream) {
    let mut buf = [0u8; 512];
    loop {
        match conn.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}
