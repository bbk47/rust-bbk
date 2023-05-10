use std::error::Error;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use h2conn::Conn as H2Conn;
use http::{Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use websocket::{OwnedMessage, ServerBuilder};

struct TunnelConn {
    tuntype: String,
    wsocket: Arc<websocket::WebSocketStream<TcpStream>>,
    tcp_socket: TcpStream,
    h2_socket: H2Conn<TcpStream>,
}

trait FrameServer {
    fn listen_conn(&self, handler: impl Fn(TunnelConn) + Send + Sync + 'static);
    fn listen_http_conn(&self, handler: impl Fn(Request<()>) -> Response<Vec<u8>> + Send + Sync + 'static);
    fn get_addr(&self) -> String;
}
