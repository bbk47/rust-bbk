use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::io::{self, BufRead};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use std::sync::mpsc::{channel, Receiver, RecvError, Sender, SyncSender, sync_channel};

use crate::utils::{get_timestamp, uuid};

use super::VirtualStream;
use crate::protocol::{self, split_frame};
use crate::protocol::{Frame, EST_FRAME, FIN_FRAME, INIT_FRAME, PING_FRAME, PONG_FRAME, RST_FRAME, STREAM_FRAME};
use crate::serializer::Serializer;
use crate::transport::Transport;

pub fn serialze_frame(serizer: Arc<Box<Serializer>>, frame: &Frame) -> Vec<u8> {
    serizer.serialize(&frame)
    //  println!("TunnelStub send frame: {} {} {}", smallframe.cid, smallframe.r#type, smallframe.data.len());
    // println!("writeing packet ==={:?}",binary_data);
}

pub struct TunnelStub {
    serizer: Arc<Box<Serializer>>,
    tsport: Arc<Box<dyn Transport + Send + Sync>>,
    streams: Arc<Mutex<HashMap<String, Arc<VirtualStream>>>>,
    streamch_send: Sender<Arc<VirtualStream>>,
    sender_send: Sender<Frame>,
    pub streamch_recv: Receiver<Arc<VirtualStream>>,
    // sender_recv: Arc<Box<Receiver<Frame>>>,
}

unsafe impl Send for TunnelStub{}
unsafe impl Sync for TunnelStub{}

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
    pub fn new( tsport: Box<dyn Transport + Send + Sync>, serizer: Arc<Box<Serializer>>) -> Self {
        let (streamch_send,  streamch_recv) = channel();
        let (sender_send,  sender_recv) = channel();

        println!("new tunnel stub worker.");
        let stub = TunnelStub {
            serizer,
            tsport: Arc::new(tsport),
            streams: Arc::new(Mutex::new(HashMap::new())),
            streamch_send: streamch_send.clone(),
            streamch_recv: streamch_recv,
            sender_send: sender_send,
            // sender_recv: Arc::new(Box::new(sender_recv)),
            // closech_recv: closech_recv,
        };
        stub.start(sender_recv);

        stub
    }

    pub fn start(&self,recv:Receiver<Frame>) {
        let sender_send_cloned = self.sender_send.clone();
        let streamch_send_cloned = self.streamch_send.clone();
        // let recver = self.sender_recv.clone();
        println!("start====stub");
        let serizer1: Arc<Box<Serializer>> = self.serizer.clone();
        let serizer2 = self.serizer.clone();
        let tsport1 = self.tsport.clone();
        let tsport2 = self.tsport.clone();
        let streams = self.streams.clone();
        // read worker
        thread::spawn(move || {
            println!("read worker started");
            'read_loop: loop {
                // println!("reading packet...");
                  // block thread
                let packet = match tsport1.read_packet() {
                    Ok(packet) => packet,
                    Err(err) => {
                        eprintln!("Transport read packet error: {:?}", err);
                        break 'read_loop;
                    }
                };
                // println!("read data==={:?}", packet);
                if let Ok(frame) = serizer2.deserialize(&packet) {
                    println!("TunnelStub read frame: {} {} {}", frame.cid, frame.r#type, frame.data.len());
                    if frame.r#type == PING_FRAME {
                        let now = get_timestamp();
                        let mut st = frame.data.clone();
                        st.extend_from_slice(now.as_bytes());
                        let cid = String::from("00000000000000000000000000000000");
                        let pong_fm = Frame::new(cid, protocol::PONG_FRAME, st);
                        if let Err(err) = sender_send_cloned.send(pong_fm) {
                            eprintln!("err:{:?}", err);
                        }
                    } else if frame.r#type == PONG_FRAME {
                        println!("pong here")
                    } else if frame.r#type == INIT_FRAME {
                        let addr = frame.data.clone();
                        let sender = sender_send_cloned.clone();
                        let stream = VirtualStream::new(frame.cid.clone(), addr, sender);
                        let st = Arc::new(stream);
                        let mut steams = streams.lock().unwrap();
                        steams.insert(frame.cid.clone(), st.clone());
                        streamch_send_cloned.send(st).unwrap();
                    } else if frame.r#type == EST_FRAME {
                        let stream_id = frame.cid.clone();
                        println!("=====est frame lock start resolve 1");
                        let steams = streams.lock().unwrap();
                        println!("=====est frame lock resolve ok 2");
                        let value = steams.get(&stream_id);
                        if let Some(st) = value {
                            let st2: Arc<VirtualStream> = (*st).clone();
                            println!("emit stream====={}, {}",&st2.addstr,&st2.cid);
                            streamch_send_cloned.send(st2).unwrap();
                            println!("emit stream ok");
                        }
                    } else if frame.r#type == STREAM_FRAME {
                        let stream_id = frame.cid.clone();
                        let steams = streams.lock().unwrap();
                        let value = steams.get(&stream_id);
                        if let Some(st) = value {
                            st.produce(&frame.data);
                        }
                    } else if frame.r#type == FIN_FRAME || frame.r#type == RST_FRAME {
                        let stream_id = frame.cid.clone();
                        let mut steams = streams.lock().unwrap();
                        let value = steams.get(&stream_id);
                        if let Some(st) = value {
                            st.close();
                            steams.remove(&stream_id);
                        }
                    } else {
                        eprintln!("eception frame type:{}", frame.r#type);
                    }
                }
            }
            println!("read worker stoped");
        });
        thread::spawn(move || {
            println!("write worker started");
            'writeloop: loop {
                  // block thread
                match recv.recv() {
                    Ok(ref fm) => {
                        let frames = split_frame(fm);
                        for smallframe in &frames {
                            let binary_data = serizer1.serialize(&smallframe);
                            println!("TunnelStub send frame: {} {} {}", smallframe.cid, smallframe.r#type, smallframe.data.len());
                            // println!("writeing packet ==={:?}",binary_data);
                            if let Err(er) = tsport2.send_packet(&binary_data) {
                                eprintln!("Failed to send frame: {:?}", er);
                                break 'writeloop;
                            }
                            // println!("write packet completed");
                        }
                    }
                    Err(err) => {
                        eprintln!("channel err {:?}", err);
                    }
                }
            }
            println!("write worker stoped");
        });
        println!("start complete.");
    }
    // fn send_frame(mut tsport: Arc<Box<dyn Transport + Send + Sync>>, serizer: Arc<Box<Serializer>>,frame: &Frame) -> io::Result<()> {
    //     let frames = split_frame(frame);
    //     for smallframe in &frames {
    //         let binary_data = serizer.serialize(&smallframe);
    //         println!("TunnelStub send frame: {} {} {}", smallframe.cid, smallframe.r#type, smallframe.data.len());
    //         tsport.send_packet(&binary_data)?;
    //     }
    //     Ok(())
    // }

    pub fn start_stream(&self, addr: &[u8]) -> String {
        let cid = uuid::get_uuidv4();
        let addrlen = addr.len();
        let data = &addr[..addrlen];
        let sender = self.sender_send.clone();
        let stream = VirtualStream::new(cid.to_owned(), data.to_vec(), sender);
        let st = Arc::new(stream);
        let mut steams = self.streams.lock().unwrap();
        steams.insert(cid.clone(), st);
        let frame = Frame::new(cid.to_owned(), protocol::INIT_FRAME, data.to_vec());
        self.sender_send.send(frame).unwrap();
        cid
    }

    // pub fn close_stream(&self, stream:&VirtualStream){
    //     let fin_frame = Frame::new(stream.cid.to_owned(), protocol::FIN_FRAME, vec![0x1,0x2] );
    //     if let Err(err) = self.sender_send.send(fin_frame) {
    //         eprintln!("err:{:?}", err);
    //     }
    // }

    pub fn set_ready(&self, stream: &VirtualStream) {
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

    pub fn accept(&self) -> Result<Arc<VirtualStream>, RecvError> {
        self.streamch_recv.recv().map_err(|e| e.into())
    }
}
