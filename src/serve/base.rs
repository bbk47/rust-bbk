use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Stream, FutureExt};

use tokio::net::TcpStream;
use tokio::sync::mpsc;

// 用于获取新连接的异步流
pub struct Incoming {
    sender: mpsc::UnboundedSender<io::Result<TcpStream>>,
    receiver: mpsc::UnboundedReceiver<io::Result<TcpStream>>,
}

impl Incoming {
    pub fn new(sender: mpsc::UnboundedSender<io::Result<TcpStream>>, receiver: mpsc::UnboundedReceiver<io::Result<TcpStream>>) -> Self {
        Incoming { sender, receiver }
    }
}

impl Stream for Incoming {
    type Item = io::Result<TcpStream>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
         // 从接收器中获取新连接，然后返回它
         match unsafe { self.receiver.poll_recv_unpin(cx) } {
            Poll::Ready(Some(res)) => {
                Poll::Ready(res)
            },
            Poll::Ready(None) => {
                Poll::Ready(None)
            },
            Poll::Pending => {
                Poll::Pending
            }
        }
    }
}

pub struct TunnelConn {
    pub tuntype: String,
    pub tcp_socket: TcpStream,
}

pub trait FrameServer {
    fn incoming(&self) -> Incoming;
    fn get_addr(&self) -> String;
}
