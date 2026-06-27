use std::sync::Arc;
use std::time::Duration;

use log::{error, info};
use tokio::net::TcpStream;

use crate::option::BbkSerOption;
use crate::proxy;
use crate::serve;
use crate::tunnel::{Session, Stream};
use crate::utils::encrypt::Encryptor;
use crate::utils::relay;
use crate::utils::socks5::AddrInfo;

pub struct BbkServer {
    opts: BbkSerOption,
    enc: Arc<Encryptor>,
}

impl BbkServer {
    pub fn new(opts: BbkSerOption) -> Self {
        let enc = Arc::new(Encryptor::new(&opts.method, &opts.password).expect("invalid encryption method"));
        BbkServer { opts, enc }
    }

    pub async fn bootstrap(self) {
        let srv = Arc::new(self);
        let enc = srv.enc.clone();
        let host = srv.opts.listen_addr.clone();
        let port = srv.opts.listen_port as u16;
        let path = srv.opts.work_path.clone();
        let crt = srv.opts.ssl_crt.clone();
        let key = srv.opts.ssl_key.clone();
        let work_mode = srv.opts.work_mode.clone();

        let handler = move |sess: Session| {
            let srv = srv.clone();
            async move {
                srv.handle_session(sess).await;
            }
        };

        if let Err(e) = serve::run(&work_mode, &host, port, &path, &crt, &key, enc, handler).await {
            error!("server exited: {}", e);
        }
    }

    async fn handle_session(self: Arc<Self>, sess: Session) {
        let sess = Arc::new(sess);
        loop {
            match sess.accept_stream().await {
                Ok(stream) => {
                    let me = self.clone();
                    tokio::spawn(async move {
                        me.handle_stream(stream).await;
                    });
                }
                Err(_) => break, // session closed
            }
        }
    }

    async fn handle_stream(self: Arc<Self>, mut stream: Stream) {
        if proxy::is_udp_marker(&stream.addr) {
            info!("stream udp-associate");
            if stream.set_ready().await.is_err() {
                return;
            }
            proxy::serve_udp(stream).await;
            return;
        }

        let info = match AddrInfo::from_buffer(&stream.addr) {
            Ok(i) => i,
            Err(_) => return,
        };
        let target = format!("{}:{}", info.host, info.port);

        let conn = match tokio::time::timeout(Duration::from_secs(10), TcpStream::connect(&target)).await {
            Ok(Ok(c)) => c,
            // Dial failure: do NOT send ready, so the client's open_stream errors
            // out (equivalent to Go's "no EST").
            _ => {
                error!("dial target failed: {}", target);
                return;
            }
        };
        conn.set_nodelay(true).ok();
        info!("connect: {}", target);

        if stream.set_ready().await.is_err() {
            return;
        }
        let mut conn = conn;
        relay(&mut stream, &mut conn).await;
    }
}
