use std::collections::HashMap;
use std::io::{self, BufRead};
use std::sync::Arc;
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::select;
use tokio::sync::mpsc::{self, UnboundedSender, UnboundedReceiver};

use crate::utils::uuid;

use super::CopyStream;
use crate::protocol::{self, split_frame};
use crate::protocol::Frame;
use crate::transport::Transport;
use crate::serializer::Serializer;

pub struct TunnelStub {
    serizer: Arc<Serializer>,
    tsport: Arc<dyn Transport>,
    streams: HashMap<String, Arc<CopyStream>>,
    streamch_send: UnboundedSender<CopyStream>,
    streamch_recv: UnboundedReceiver<CopyStream>,
    sender_send: UnboundedSender<Frame>,
    sender_recv: UnboundedReceiver<Frame>,
    closech_send: UnboundedSender<u8>,
    closech_recv: UnboundedReceiver<u8>,
    pong_func: Option<Box<dyn FnMut(i64, i64) + Send + 'static>>,
}

impl TunnelStub {
    pub fn new(tsport: Arc<dyn Transport>, serizer: Arc<Serializer>) -> io::Result<Self> {
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
            pong_func: None,
        };

        let tsport_cloned = tsport.clone();
        let serizer_cloned = serizer.clone();
        let sender_send_cloned = sender_send.clone();
        thread::spawn(move || Self::read_worker(tsport_cloned, serizer_cloned, sender_send_cloned, closech_recv));
        thread::spawn(move || Self::write_worker(sender_recv, closech_recv));

        Ok(stub)
    }

    fn read_worker(tsport: Arc<dyn Transport>, serizer: Arc<Serializer>, sender_send: UnboundedSender<Frame>, closech: UnboundedReceiver<u8>) {
        println!("TunnelStub read worker started");
        let cid = String::from("00000000000000000000000000000000");
        let mut last_ping = Instant::now();

        'read_loop: loop {
            select!(
            recv(closech.clone()) -> _ => {
                println!("TunnelStub read worker stopping due to close signal");
                break 'read_loop;
            }
            default => {
                let packet = match tsport.read_packet() {
                    Ok(packet) => packet,
                    Err(err) => {
                        log::error!("Transport read packet error: {:?}", err);
                        break 'read_loop;
                    }
                };

                if let Ok(frame) = serizer.deserialize(&packet) {
                    println!(
                        "TunnelStub read frame: {} {} {}",
                        frame.cid,
                        frame.frame_type,
                        frame.content_length()
                    );

                    match frame.frame_type {
                        FrameType::Ping => {
                            let now = Instant::now();
                            let duration = now.duration_since(last_ping).as_millis() as i64;
                            last_ping = now;

                            let data = [&frame.content(), &utils::i64_to_bytes_le(duration)]
                            .concat()
                            .to_vec();
                            let pong = Frame::new_pong(cid.clone(), data);
                            sender_send.send(pong).unwrap();
                        }
                        FrameType::Pong => {
                            let (up, down) = match utils::parse_ping_pong(&frame.content()) {
                                Ok((up, down)) => (up, down),
                                Err(_) => continue,
                            };
                            if let Some(ref mut handler) = &mut self.pong_func {
                                handler(up, down);
                            }
                        }
                        FrameType::Init => {
                            let stream = Arc::new(CopyStream::new(frame.cid.clone(), &frame.content()));
                            self.streams.insert(frame.cid.clone(), stream.clone());
                            sender_send.send(frame).unwrap();
                            self.streamch.send(stream).unwrap();
                        }
                        FrameType::CopyStream => {
                            if let Some(stream) = self.streams.get(&frame.cid) {
                                stream.produce(&frame.content());
                            }
                        }
                        FrameType::Fin | FrameType::Rst => {
                            if let Some(stream) = self.streams.remove(&frame.cid) {
                                stream.close();
                                sender_send.send(frame).unwrap();
                            }
                        }
                        FrameType::Est => {
                                if let Some(stream) = self.streams.get(&frame.cid) {
                                    self.streamch.send(stream.clone()).unwrap();
                                }
                        }
                        _ => {
                                log::warn!("Unexpected frame type: {}", frame.frame_type);
                        }
                    }
                }
            });
        }
        println!("TunnelStub read worker stopped");
    }

    fn write_worker(sender_recv: UnboundedReceiver<Frame>, close_recv: UnboundedReceiver<u8>) {
        println!("TunnelStub write worker started");
        loop {
            select!(
                recv(sender_recv) -> ref frame => {
                    if let Err(err) = self.send_frame(frame) {
                        log::error!("Failed to send frame: {:?}", err);
                        break;
                    }
                },
                recv(close_recv.clone()) -> _ => {
                    println!("TunnelStub write worker stopping due to close signal");
                    break;
                }
            );
        }
        println!("TunnelStub write worker stopped");
    }

    fn send_frame(&self, frame: &Frame) -> io::Result<()> {
        let frames = split_frame(frame);
        for smallframe in &frames {
            let binary_data = self.serizer.serialize(&smallframe);
            println!("TunnelStub send frame: {} {} {}", smallframe.cid, smallframe.r#type, smallframe.data.len());
            self.tsport.send_packet(&binary_data)?;
        }
        Ok(())
    }

    pub fn notify_pong<F>(&mut self, mut handler: F)
    where
        F: FnMut(i64, i64) + Send + 'static,
    {
        self.pong_func = Some(Box::new(handler));
    }

    pub fn start_stream(&self, addr: &[u8]) -> Arc<CopyStream> {
        let cid = uuid::get_uuidv4();
        let addrlen = addr.len();
        let stream = Arc::new(CopyStream::new(cid.clone(), addr[...addrlen],));
        self.streams.insert(cid, stream.clone());
        let frame =Frame::new(1, cid, protocol::INIT_FRAME, addr[...addrlen])
        self.sender_send.send(frame).unwrap();
        stream
    }

    pub fn set_ready(&self, stream: &CopyStream) {
        let frame = Frame::new_est(stream.cid.clone(), stream.addr.clone());
        self.sender_send.send(frame).unwrap();
    }

    pub fn ping(&self) {
        let cid = String::from("00000000000000000000000000000000");
        let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();
        let data = [&now.to_be_bytes()].concat().to_vec();
        let frame = Frame::new_ping(cid, data);
        self.sender_send.send(frame).unwrap();
    }

    pub fn accept(&self) -> Option<Arc<CopyStream>> {
        match self.streamch_recv.recv() {
            Ok(stream) => Some(stream),
            Err(_) => None,
        }
    }
}
