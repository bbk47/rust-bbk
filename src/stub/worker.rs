use log::debug;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::{mem, thread};

use std::sync::mpsc::{channel, sync_channel, Receiver, RecvError, Sender, SyncSender, TryRecvError};

use crate::utils::emiter::EventManager;
use crate::utils::{get_timestamp_bytes, uuid};

use super::VirtualStream;
use crate::protocol::{self, split_frame};
use crate::protocol::{Frame, EST_FRAME, FIN_FRAME, INIT_FRAME, PING_FRAME, PONG_FRAME, RST_FRAME, STREAM_FRAME};
use crate::serializer::Serializer;
use crate::transport::Transport;

pub struct PongMessage {
    pub stime: i64,
    pub atime: i64,
}

pub struct TunnelStub {
    serizer: Arc<Serializer>,
    tsport: Arc<Box<dyn Transport + Send + Sync>>,
    streams: Arc<Mutex<HashMap<String, Arc<VirtualStream>>>>,
    streamch_send: Sender<Option<Arc<VirtualStream>>>,
    fm_send: Sender<Option<Frame>>,
    fm_recv: Receiver<Option<Frame>>,
    pub emiter: Arc<Mutex<EventManager<PongMessage>>>,
    pub streamch_recv: Receiver<Option<Arc<VirtualStream>>>,
}

unsafe impl Send for TunnelStub {}
unsafe impl Sync for TunnelStub {}

impl TunnelStub {
    pub fn new(tsport: Box<dyn Transport + Send + Sync>, serizer: Arc<Serializer>) -> Self {
        let (streamch_send, streamch_recv) = channel();
        let (fm_send, fm_recv) = channel();

        // println!("new tunnel stub worker.");
        let stub = TunnelStub {
            serizer,
            tsport: Arc::new(tsport),
            streams: Arc::new(Mutex::new(HashMap::new())),
            streamch_send: streamch_send,
            streamch_recv: streamch_recv,
            fm_send: fm_send,
            fm_recv: fm_recv,
            emiter: Arc::new(Mutex::new(EventManager::new())),
            // closech_recv: closech_recv,
        };

        stub
    }

    pub fn start(&self) {
        println!("start start!!");
        thread::scope(|s| {
            s.spawn(move || self.read_worker());
            s.spawn(move || self.write_worker());
            println!("start after1!!");
        });
        self.streamch_send.send(None);
        println!("start after!!2");
    }
    fn read_worker(&self) {
        println!("read worker started");
        'read_loop: loop {
            // println!("reading packet...");
            // block thread
            let packet = match self.tsport.read_packet() {
                Ok(packet) => packet,
                Err(err) => {
                    eprintln!("Transport read packet error: {:?}", err);
                    self.fm_send.send(None).unwrap();
                    break 'read_loop;
                }
            };
            // println!("read data==={:?}", packet);
            if let Ok(frame) = self.serizer.deserialize(&packet) {
                // println!("TunnelStub read frame: {} {} {}", frame.cid, frame.r#type, frame.data.len());
                if frame.r#type == PING_FRAME {
                    let mut st = frame.data.clone();
                    let data2 = get_timestamp_bytes();
                    st.extend_from_slice(&data2);
                    let cid = String::from("00000000000000000000000000000000");
                    let pong_fm = Frame::new(cid, protocol::PONG_FRAME, st);
                    if let Err(err) = self.fm_send.send(Some(pong_fm)) {
                        eprintln!("err:{:?}", err);
                    }
                } else if frame.r#type == PONG_FRAME {
                    // 当需要通知外部时，调用回调函数并传递数据
                    let datas = frame.data.as_slice();
                    let stime = std::str::from_utf8(&datas[..13]).unwrap();
                    let atime = std::str::from_utf8(&datas[13..]).unwrap();
                    let message = PongMessage {
                        stime: stime.parse().unwrap(),
                        atime: atime.parse().unwrap(),
                    };
                    self.emiter.lock().unwrap().publish("pong", &message);
                } else if frame.r#type == INIT_FRAME {
                    let addr = frame.data.clone();
                    let sender = self.fm_send.clone();
                    let stream = VirtualStream::new(frame.cid.clone(), addr, sender);
                    let mut steams = self.streams.lock().unwrap();
                    let st = Arc::new(stream);
                    steams.insert(frame.cid.clone(), st.clone());
                    self.streamch_send.send(Some(st)).unwrap();
                } else if frame.r#type == EST_FRAME {
                    let stream_id = frame.cid.clone();
                    // println!("=====est frame lock start resolve 1");
                    let steams = self.streams.lock().unwrap();
                    // println!("=====est frame lock resolve ok 2");
                    let value = steams.get(&stream_id);
                    if let Some(st) = value {
                        // println!("emit stream====={}, {}",&st2.addstr,&st2.cid);
                        self.streamch_send.send(Some(st.clone())).unwrap();
                        // println!("emit stream ok");
                    }
                } else if frame.r#type == STREAM_FRAME {
                    let stream_id = frame.cid.clone();
                    let steams = self.streams.lock().unwrap();
                    let value = steams.get(&stream_id);
                    if let Some(st) = value {
                        st.produce(&frame.data);
                    }
                } else if frame.r#type == FIN_FRAME || frame.r#type == RST_FRAME {
                    let stream_id = frame.cid.clone();
                    let mut steams = self.streams.lock().unwrap();
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
        println!("read  worker stoped");
    }
    fn write_worker(&self) {
        println!("write worker started");
        'write_loop: loop {
            // block thread
            match self.fm_recv.recv() {
                Ok(ref ret) => {
                    match ret {
                        None => {
                            // close sigle from read worker
                            break 'write_loop;
                        }
                        Some(fm) => {
                            let frames = split_frame(fm);
                            for smallframe in &frames {
                                let binary_data = self.serizer.serialize(&smallframe);
                                debug!("TunnelStub write frame: {} {} {}", smallframe.cid, smallframe.r#type, smallframe.data.len());
                                // println!("writeing packet ==={:?}",binary_data);
                                if let Err(er) = self.tsport.send_packet(&binary_data) {
                                    eprintln!("Failed to send frame: {:?}", er);
                                    break 'write_loop;
                                }
                                // println!("write packet completed");
                            }
                        }
                    }
                }

                Err(err) => {
                    eprintln!("err:{:?}", err);
                    break 'write_loop;
                }
            }
        }
        println!("write worker stoped");
    }

    pub fn start_stream(&self, addr: &[u8]) -> String {
        let cid = uuid::get_uuidv4();
        let addrlen = addr.len();
        let data = &addr[..addrlen];
        let sender = self.fm_send.clone();
        let stream = VirtualStream::new(cid.to_owned(), data.to_vec(), sender);
        let mut steams = self.streams.lock().unwrap();
        steams.insert(cid.clone(), Arc::new(stream));
        let frame = Frame::new(cid.to_owned(), protocol::INIT_FRAME, data.to_vec());
        self.fm_send.send(Some(frame)).unwrap();
        cid
    }

    // pub fn close_stream(&self, stream:&VirtualStream){
    //     let fin_frame = Frame::new(stream.cid.to_owned(), protocol::FIN_FRAME, vec![0x1,0x2] );
    //     if let Err(err) = self.fm_send.send(fin_frame) {
    //         eprintln!("err:{:?}", err);
    //     }
    // }
    pub fn set_ready(&self, stream: &VirtualStream) {
        let data = stream.addr.clone();
        let frame = Frame::new(stream.cid.clone(), protocol::EST_FRAME, data);
        self.fm_send.send(Some(frame)).unwrap();
    }

    pub fn ping(&self) {
        let data = get_timestamp_bytes();
        let cid = String::from("00000000000000000000000000000000");
        let frame = Frame::new(cid, protocol::PING_FRAME, data);
        self.fm_send.send(Some(frame)).unwrap();
    }

    pub fn accept(&self) -> Result<Option<Arc<VirtualStream>>, RecvError> {
        self.streamch_recv.recv().map_err(|e| e.into())
    }
}
