use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::io::{self, BufRead};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::select;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;

use crate::utils::{get_timestamp, uuid};

use super::CopyStream;
use crate::protocol::Frame;
use crate::protocol::{self, split_frame};
use crate::serializer::Serializer;
use crate::transport::Transport;

pub struct TunnelStub {
    serizer: Arc<Box<Serializer>>,
    tsport: Arc<Box<dyn Transport + Send + Sync>>,
    // streams: HashMap<String, Arc<CopyStream>>,
    streamch_send: UnboundedSender<CopyStream>,
    sender_send: UnboundedSender<Frame>,
    streamch_recv: UnboundedReceiver<CopyStream>,
    sender_recv: Arc<Mutex<UnboundedReceiver<Frame>>>,
}

// impl DerefMut for TunnelStub {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self
//     }
// }

// impl Deref for TunnelStub {
//     type Target = Vec<u8>;

//     fn deref(&self) -> &Self::Target {
//         &self.tunnels
//     }
// }

impl TunnelStub {
    pub fn new(mut tsport: Box<dyn Transport + Send + Sync>, serizer: Arc<Box<Serializer>>) -> Self {
        let (streamch_send, mut streamch_recv) = mpsc::unbounded_channel();
        let (sender_send, mut sender_recv) = mpsc::unbounded_channel();

        println!("new tunnel stub worker.");
        let stub = TunnelStub {
            serizer,
            tsport: Arc::new(tsport),
            // streams: HashMap::new(),
            streamch_send: streamch_send.clone(),
            sender_send: sender_send.clone(),
            streamch_recv: streamch_recv,
            sender_recv: Arc::new(Mutex::new(sender_recv)),
            // closech_recv: closech_recv,
        };

        stub
    }

    pub fn start(&self) {
        // let sender_send_cloned = sender_send.clone();
        let recver = self.sender_recv.clone();
        println!("start====stub");
        let serizer1: Arc<Box<Serializer>> = self.serizer.clone();
        let serizer2 = self.serizer.clone();
        let tsport1 = self.tsport.clone();
        let tsport2 = self.tsport.clone();

        // read worker
        tokio::spawn(async move {
            println!("read worker started");
            'read_loop: loop {
                sleep(Duration::from_millis(1000)).await;
                // thread::sleep(Duration::from_millis(1000));
                println!("read tick ===");
                println!("read packet...");
                let packet = match tsport1.read_packet() {
                    Ok(packet) => packet,
                    Err(err) => {
                        eprintln!("Transport read packet error: {:?}", err);
                        break 'read_loop;
                    }
                };
                println!("read data===");
                if let Ok(frame) = serizer2.deserialize(&packet) {
                    // println!(
                    //     "TunnelStub read frame: {} {} {}",
                    //     frame.cid,
                    //     frame.frame_type,
                    //     frame.content_length()
                    // );

                    println!("recv frame type:{}", frame.r#type);
                }
            }
            println!("read worker stoped");
        });
        tokio::spawn(async move {
            println!("write worker started");
            'writeloop: loop {
                sleep(Duration::from_millis(1000)).await;
                // thread::sleep(Duration::from_millis(1000));
                println!("write tick ===");
                let mut rec = recver.lock().unwrap();
                match rec.try_recv() {
                    Ok(ref fm) => {
                        let frames = split_frame(fm);
                        for smallframe in &frames {
                            let binary_data = serizer1.serialize(&smallframe);
                            println!("TunnelStub send frame: {} {} {}", smallframe.cid, smallframe.r#type, smallframe.data.len());
                            println!("resolve tsparc2");
                            // let mut ts = tsparc2.borrow_mut();
                            if let Err(er) = tsport2.send_packet(&binary_data) {
                                eprintln!("Failed to send frame: {:?}", er);
                                break 'writeloop;
                            }
                        }
                        println!("send====after")
                    }
                    Err(TryRecvError::Disconnected) => {
                        println!("channel is closed 1");
                    }
                    Err(TryRecvError::Empty) => {
                        println!("channel is empty.");
                    }
                }
            }
            println!("write worker stoped");
        });
        println!("start complete.");
    }
    // fn send_frame(&mut self, frame: &Frame) -> io::Result<()> {
    //     let frames = split_frame(frame);
    //     for smallframe in &frames {
    //         let binary_data = self.serizer.serialize(&smallframe);
    //         println!("TunnelStub send frame: {} {} {}", smallframe.cid, smallframe.r#type, smallframe.data.len());
    //         self.tsport.send_packet(&binary_data)?;
    //     }
    //     Ok(())
    // }

    pub fn start_stream(&self, addr: &[u8]) -> Arc<CopyStream> {
        let cid = uuid::get_uuidv4();
        let addrlen = addr.len();
        let data = &addr[..addrlen];
        let stream = CopyStream::new(cid.to_owned(), data.to_vec());
        let stwrap = Arc::new(stream);
        // self.streams.insert(cid, stwrap);
        let frame = Frame::new(cid.to_owned(), protocol::INIT_FRAME, data.to_vec());
        println!("send to transport====>>>>11");
        self.sender_send.send(frame).unwrap();
        println!("send to transport====>>>>22");
        stwrap
    }

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

    pub fn accept(&mut self) -> Result<CopyStream, TryRecvError> {
        match self.streamch_recv.try_recv() {
            Ok(st) => Ok(st),
            Err(TryRecvError::Disconnected) => Err(TryRecvError::Disconnected),
            Err(TryRecvError::Empty) => Err(TryRecvError::Empty),
        }
    }
    // pub async fn accept(&mut self) -> Result<CopyStream, TryRecvError> {
    //     let st = self.streamch_recv.recv().await;
    //     match st {
    //         Some(st) => Ok(st),
    //         None => Err(TryRecvError::Empty),
    //     }
    // }
}
