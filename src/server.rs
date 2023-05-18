use crate::option::BbkSerOption;
use crate::serve;
use crate::serve::{FrameServer, TunnelConn};
use crate::utils::logger::{LogLevel, Logger};
use std::error::Error;

pub struct BbkServer {
    opts: BbkSerOption,
    logger: Logger,
}

impl BbkServer {
    pub fn new(opts: BbkSerOption) -> Self {
        println!("server new====");
        let logger = Logger::new(LogLevel::Info);
        BbkServer { opts: opts, logger }
    }

    pub fn handle_connection(&self, conn: &TunnelConn) -> Result<(), Box<dyn Error>> {
        // Handle connection logic here.
        // println!("Received a connection from {}", tunnel_conn.tcp_socket.peer_addr().unwrap());
        Ok(())
    }

    fn init_server(&self) {
        if self.opts.listen_port <= 1024 && self.opts.listen_port >= 65535 {
            panic!("invalid port: {}", self.opts.listen_port);
        }
        let port = self.opts.listen_port as u16;
        let addr = format!("{}:{}", &self.opts.listen_addr, &port);
        let server = match &self.opts.work_mode[..] {
            "tcp" => serve::new_abc_tcp_server(&self.opts.listen_addr, port),
            // "tls" => serve::new_abc_tls_server(&addr, &self.opts.listen_port, &self.opts.ssl_crt, &self.opts.ssl_key),
            // "ws" => server::new_abc_wss_server(&addr, &self.opts.work_path),
            // "h2" => server::new_abc_http2_server(&addr, &self.opts.work_path, &self.opts.ssl_crt, &self.opts.ssl_key),
            _ => {
                self.logger.info(&format!("unsupported work mode: {}", &self.opts.work_mode));
                Err("unsupported work mode".into())
            }
        }
        .unwrap();
        self.logger.info(&format!("server listen on {:?}", server.get_addr()));
        // server.listen_conn(|t: &TunnelConn| self.handle_connection(t));
        server .listen_conn(|tunnel_conn| {
            println!("Received a connection from {}", tunnel_conn.tcp_socket.peer_addr().unwrap());
        }).unwrap();
    }

    // fn init_serizer(&self) -> Result<Serializer, Box<dyn Error>> {
    //     Serializer::new(&self.opts.method, &self.opts.password)
    //         .map_err(|e| -> Box<dyn Error> { Box::new(e) })
    // }

    pub fn bootstrap(&self) {
        self.init_server();
    }
}
