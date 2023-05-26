use crate::proxy::{self, ProxySocket};
use crate::stub::{self, Stream};
use crate::toolbox::{self, AddrInfo};
use crate::transport::{self, CreateTransport, Option as TunnelOpts};
use crate::utils::{self, LogLevel};
use log::{error, info};
use retry::{delay::Exponential, retry};
use std::time::Duration;
use std::{collections::HashMap, net::TcpListener, sync::mpsc::channel, thread};
use tokio::sync::mpsc;

const TUNNEL_INIT: u8 = 0; // 道路隧道状态常量
const TUNNEL_OK: u8 = 1;
const TUNNEL_DISCONNECT: u8 = 2;

struct BrowserObj {
    cid: String,
    proxy_socket: proxy::ProxySocket,
    stream_ch: mpsc::Sender<stub::Stream>,
}

struct Client {
    opts: Option,
    logger: utils::Logger,
    tunnel_opts: TunnelOpts,
    req_ch: mpsc::Receiver<BrowserObj>,
    retry_count: u8,
    tunnel_status: u8,
    stub_client: stub::TunnelStub,
    transport: transport::Transport,
    last_pong: u64,
    browser_proxy: HashMap<String, BrowserObj>, // 线程共享变量
}

impl Client {
    pub fn new(opts: Option) -> Self {
        let logger = utils::new_logger("C", opts.log_level);
        let (tx, rx) = mpsc::unbounded_channel(); // 使用tokio的mpsc替代crossbeam_channel
        Self {
            opts,
            logger,
            tunnel_opts: opts.tunnel_opts.clone(),
            req_ch: rx,
            retry_count: 0,
            tunnel_status: TUNNEL_INIT,
            stub_client: stub::TunnelStub::default(),
            transport: transport::Transport::default(),
            last_pong: time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_secs(),
            browser_proxy: HashMap::new(),
        }
    }

    pub fn bootstrap(&mut self) -> Result<()> {
        self.service_worker();
        self.init_server().context("Failed to initialize server")?;
        Ok(())
    }

    fn setup_ws_connection(&mut self) -> Result<()> {
        let tun_opts = self.tunnel_opts.clone();
        self.logger.info(format!("creating {} tunnel", tun_opts.protocol));
        let result = retry(
            Exponential::from_millis(500) // 指数计算重试延迟
                .map(|x| Duration::from_millis(x))
                .take(5) // 最多重试5次
                .retry_if(|error| {
                    // 尝试捕获任何错误，并返回true以进行重试
                    error!("setup tunnel failed!{:?}", error);
                    true
                }),
            || {
                self.transport = CreateTransport(tun_opts.clone())?;
                self.stub_client = stub::TunnelStub::new(&self.transport);
                self.stub_client.notify_pong(|up, down| {
                    self.logger.info(format!("tunnel health！ up:{}ms, down:{}ms rtt:{}ms", up, down, up + down));
                });
                self.tunnel_status = TUNNEL_OK;
                self.logger.debug("create tunnel success!");
                Ok(())
            },
        );
        result.context(format!("Failed to create {} tunnel", tun_opts.protocol))
    }

    fn listen_stream(&mut self) -> Result<()> {
        loop {
            let stream = self.stub_client.accept()?;
            let brower_obj = self.browser_proxy.get(stream.cid());
            if let Some(brower_obj) = brower_obj {
                let ch = &brower_obj.stream_ch;
                match ch.send(stream).await {
                    // 使用await替代send
                    Ok(_) => (),
                    Err(_) => {
                        self.browser_proxy.remove(stream.cid());
                    }
                }
            }
        }
    }

    fn bind_proxy_socket(&mut self, socket: proxy::ProxySocket) -> Result<()> {
        let remote_address = match AddrInfo::parse(socket.get_addr()) {
            Ok(addr_info) => format!("{}:{}", addr_info.addr, addr_info.port),
            Err(err) => {
                self.logger.error(format!("parse addr info err:{:?}", err));
                return Ok(());
            }
        };
        let browser_obj = BrowserObj {
            proxy_socket: socket,
            cid: "".to_string(),
            stream_ch: mpsc::channel(10).0, // 使用
        };
        self.req_ch.send(browser_obj)?;
        if let Some(brower_obj) = self.req_ch.recv_timeout(Duration::from_secs(15)).await {
            self.logger.info(format!("EST success:{}", remote_address));
            let proxy_socket = brower_obj.proxy_socket;
            let stream = match self.stub_client.start_stream(&proxy_socket.get_addr()) {
                Ok(stream) => stream,
                Err(_) => {
                    self.logger.warn(format!("Failed to start stream for {}", remote_address));
                    return Ok(());
                }
            };
            browser_obj.cid = stream.cid().to_string();
            self.browser_proxy.insert(browser_obj.cid.clone(), browser_obj);
            stub::relay(proxy_socket, stream).await?; // 使用await替代relay
            self.browser_proxy.remove(&browser_obj.cid);
        } else {
            self.logger.warn(format!("connect {} timeout 15000ms exceeded!", remote_address));
        }
        Ok(())
    }

    fn service_worker(&mut self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(10); // 使用tokio的mpsc
        let mut proxy_server_tx = tx.clone();
        std::thread::spawn(move || {
            for req in rx.recv().iter() {
                let result = self.stub_client.start_stream(&req.proxy_socket.get_addr());
                match result {
                    Ok(stream) => {
                        let browser_obj = BrowserObj {
                            proxy_socket: req.proxy_socket,
                            cid: stream.cid().to_string(),
                            stream_ch: mpsc::channel(10).0,
                        };
                        self.browser_proxy.insert(browser_obj.cid.clone(), browser_obj);
                        tx.send(browser_obj).unwrap();
                    }
                    Err(_) => {
                        self.retry_count = self.retry_count.wrapping_add(1);
                        if self.retry_count > 3 {
                            self.tunnel_status = TUNNEL_DISCONNECT;
                            return;
                        } else {
                            proxy_server_tx.send(req).await.unwrap(); // 使用await替代send
                        }
                    }
                }
            }
        });

        loop {
            if self.tunnel_status != TUNNEL_OK {
                if let Err(err) = self.setup_ws_connection() {
                    error!("Failed to setup ws connection: {:?}", err);
                    self.tunnel_status = TUNNEL_DISCONNECT;
                    continue;
                }
            }
            match self.req_ch.recv().await {
                Some(ref request) => {
                    let tx = tx.clone();
                    let proxy_socket = request.proxy_socket.try_clone()?;
                    tokio::spawn(async move {
                        // 使用tokio::spawn替代thread::spawn
                        if let Err(err) = tx
                            .send(BrowserObj {
                                proxy_socket,
                                cid: "".to_string(),
                                stream_ch: mpsc::channel(10).0,
                            })
                            .await
                        {
                            error!("Error sending to service worker channel: {:?}", err);
                        }
                    });
                }
                None => (),
            }
        }
    }

    fn init_proxy_server(&mut self, port: u16, is_connect: bool) -> Result<()> {
        let listener = TcpListener::bind(&self.opts.listen_addr, port)?;
        self.logger.info(format!("proxy server listen on {}", listener.local_addr()?));
        for stream in listener.incoming() {
            match stream {
                Ok(conn) => {
                    let tx = self.req_ch.clone();
                    tokio::spawn(async move {
                        // 使用tokio::spawn替代thread::spawn
                        let result = match is_connect {
                            true => proxy::new_connect_proxy(conn),
                            false => proxy::new_socks5_proxy(conn),
                        };
                        if let Ok(proxy_socket) = result {
                            tx.send(BrowserObj {
                                proxy_socket,
                                cid: "".to_string(),
                                stream_ch: mpsc::channel(10).0,
                            })
                            .await
                            .unwrap(); // 使用await替代send
                        }
                    });
                }
                Err(err) => {
                    self.logger.error(format!("Failed to accept connection: {:?}", err));
                }
            }
        }
        Ok(())
    }

    fn init_server(&mut self) -> Result<()> {
        let opt = &self.opts;
        self.init_proxy_server(opt.listen_port, false)?;
        Ok(())
    }
}
