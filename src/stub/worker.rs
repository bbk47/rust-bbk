use std::collections::HashMap;
use std::error::Error;
use std::io::{self, BufRead};
use std::sync::Arc;
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::select;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::utils::{get_timestamp, uuid};

use super::CopyStream;
use crate::protocol::Frame;
use crate::protocol::{self, split_frame};
use crate::serializer::Serializer;
use crate::transport::Transport;

pub struct TunnelStub<'a> {
    serizer: &'a Arc<Serializer>,
    tsport: Arc<dyn Transport>,
    streams: HashMap<String, Arc<CopyStream>>,
    streamch_send: UnboundedSender<Arc<CopyStream>>,
    streamch_recv: UnboundedReceiver<Arc<CopyStream>>,
    sender_send: UnboundedSender<Frame>,
    sender_recv: UnboundedReceiver<Frame>,
    closech_send: UnboundedSender<()>,
    closech_recv: UnboundedReceiver<()>,
}

impl<'a> TunnelStub<'a> {
    pub fn new(tsport: Arc<dyn Transport + Send + Sync>, serizer: &'a Arc<Serializer>) -> io::Result<Self> {
        let (streamch_send, streamch_recv) = mpsc::unbounded_channel();
        let (sender_send, sender_recv) = mpsc::unbounded_channel();
        let (closech_send, closech_recv) = mpsc::unbounded_channel();

        let stub = TunnelStub {
            serizer,
            tsport,
            streams: HashMap::new(),
            streamch_send: streamch_send,
            sender_send: sender_send,
            closech_send: closech_send,
            streamch_recv: streamch_recv,
            sender_recv: sender_recv,
            closech_recv: closech_recv,
        };

        // let tsport_cloned = tsport.clone();
        // let serizer_cloned = serizer.clone();
        // let sender_send_cloned = sender_send.clone();

        // let readworker = Self::read_worker(&stub, tsport_cloned, serizer_cloned, sender_send_cloned, closech_recv);
        // let writeworker = Self::write_worker(&stub, sender_recv, closech_recv);
        // tokio::spawn(readworker);
        // tokio::spawn(writeworker);

        Ok(stub)
    }

    fn read_worker(stub: &'a TunnelStub, tsport: Arc<dyn Transport>, serizer: Arc<Serializer>, sender_send: UnboundedSender<Frame>, closech: UnboundedReceiver<()>) {
        println!("TunnelStub read worker started");
        let cid = String::from("00000000000000000000000000000000");
        let mut last_ping = Instant::now();

        // 'read_loop: loop {
        //     select!(
        //         _ = closech.recv() => {
        //         println!("TunnelStub read worker stopping due to close signal");
        //         break 'read_loop;
        //         },
        //     _ = async {} => {
        //         let packet = match tsport.read_packet() {
        //             Ok(packet) => packet,
        //             Err(err) => {
        //                 eprintln!("Transport read packet error: {:?}", err);
        //                 break 'read_loop;
        //             }
        //         };

        //         if let Ok(frame) = serizer.deserialize(&packet) {
        //             // println!(
        //             //     "TunnelStub read frame: {} {} {}",
        //             //     frame.cid,
        //             //     frame.frame_type,
        //             //     frame.content_length()
        //             // );

        //             println!("recv frame type:{}",frame.r#type);
        //         }
        //     })
        // }
        println!("TunnelStub read worker stopped");
    }

    fn write_worker(stub: &'a TunnelStub, sender_recv: UnboundedReceiver<Frame>, close_recv: UnboundedReceiver<()>) {
        println!("TunnelStub write worker started");
        // loop {
        //     select!(
        //         frame = sender_recv.recv() => {
        //             if let Err(err) = stub.send_frame(frame) {
        //                 eprintln!("Failed to send frame: {:?}", err);
        //                 break;
        //             }
        //         },
        //         _ = close_recv.recv() => {
        //             println!("TunnelStub write worker stopping due to close signal");
        //             break;
        //         }
        //     );
        // }
        println!("TunnelStub write worker stopped");
    }

    // fn send_frame(&self, frame: &Frame) -> io::Result<()> {
    //     let frames = split_frame(frame);
    //     for smallframe in &frames {
    //         let binary_data = self.serizer.serialize(&smallframe);
    //         println!("TunnelStub send frame: {} {} {}", smallframe.cid, smallframe.r#type, smallframe.data.len());
    //         self.tsport.send_packet(&binary_data)?;
    //     }
    //     Ok(())
    // }


    // pub fn start_stream(&self, addr: &[u8]) -> Arc<CopyStream> {
    //     let cid = uuid::get_uuidv4();
    //     let addrlen = addr.len();
    //     let data = &addr[..addrlen];
    //     let stream = CopyStream::new(cid, data.to_vec());
    //     let arcstream = Arc::new(stream);
    //     self.streams.insert(cid, arcstream);
    //     let frame = Frame::new(cid, protocol::INIT_FRAME, data.to_vec());
    //     self.sender_send.send(frame).unwrap();
    //     arcstream
    // }

    pub fn set_ready(&self, stream: &CopyStream) {
        let data = stream.addr.clone();
        let frame = Frame::new(stream.cid.clone(), protocol::EST_FRAME, data);
        self.sender_send.send(frame).unwrap();
    }

    pub fn ping(&self) {
        let cid = String::from("00000000000000000000000000000000");
        let now = get_timestamp();
        let data = now.as_bytes().to_vec();
        let cid = String::from("00000000000000000000000000000000");
        let frame = Frame::new(cid, protocol::PING_FRAME, data);
        self.sender_send.send(frame).unwrap();
    }

    pub async fn accept(&mut self) -> Result<Arc<CopyStream>, Box<dyn Error>> {
        select!(
            ret = self.streamch_recv.recv() => {
                match ret {
                    Some(stream) => Ok(stream),
                    None => {
                        Err("recv None".into())
                    },
                }
            },
            _ = self.closech_recv.recv() => {
                println!("TunnelStub write worker stopping due to close signal");
                Err("closed transport".into())
            }
        )
    }
}
