use std::collections::HashMap;
use std::io::{self, BufRead};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crate::protocol::frame::{self, Frame, FrameType};
use crate::protocol::segment::FrameSegment;
use crate::serializer::Serializer;
use crate::stream::Stream;
use crate::transport::Transport;
use crate::utils;

pub struct TunnelStub {
    serizer: Arc<Serializer>,
    tsport: Arc<dyn Transport>,
    streams: HashMap<String, Arc<Stream>>,
    streamch: crossbeam_channel::Receiver<Arc<Stream>>,
    sendch: crossbeam_channel::Sender<Frame>,
    closech: crossbeam_channel::Receiver<()>,
    pong_func: Option<Box<dyn FnMut(i64, i64) + Send + 'static>>,
}

impl TunnelStub {
    pub fn new(tsport: Arc<dyn Transport>, serizer: Arc<Serializer>) -> io::Result<Self> {
        let (stream_send, stream_recv) = crossbeam_channel::unbounded();
        let (send_send, send_recv) = crossbeam_channel::unbounded();
        let (close_send, close_recv) = crossbeam_channel::unbounded();

        let stub = TunnelStub {
            serizer,
            tsport,
            streams: HashMap::new(),
            streamch: stream_recv,
            sendch: send_send,
            closech: close_recv,
            pong_func: None,
        };

        let tsport_cloned = tsport.clone();
        let serizer_cloned = serizer.clone();
        let sendch_cloned = send_send.clone();
        let closech_cloned = close_recv.clone();
        thread::spawn(move || Self::read_worker(tsport_cloned, serizer_cloned, sendch_cloned, closech_cloned));

        let sendch_cloned = send_recv.clone();
        thread::spawn(move || Self::write_worker(sendch_cloned, close_recv));

        Ok(stub)
    }

    fn read_worker(tsport: Arc<dyn Transport>, serizer: Arc<Serializer>, sendch: crossbeam_channel::Sender<Frame>, closech: crossbeam_channel::Receiver<()>) {
        log::debug!("TunnelStub read worker started");
        let cid = frame::ID_ZERO.to_string();
        let mut last_ping = Instant::now();

        'read_loop: loop {
            select!(
            recv(closech.clone()) -> _ => {
                log::debug!("TunnelStub read worker stopping due to close signal");
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
                    log::debug!(
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
                            sendch.send(pong).unwrap();
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
                            let stream = Arc::new(Stream::new(frame.cid.clone(), &frame.content()));
                            self.streams.insert(frame.cid.clone(), stream.clone());
                            sendch.send(frame).unwrap();
                            self.streamch.send(stream).unwrap();
                        }
                        FrameType::Stream => {
                            if let Some(stream) = self.streams.get(&frame.cid) {
                                stream.produce(&frame.content());
                            }
                        }
                        FrameType::Fin | FrameType::Rst => {
                            if let Some(stream) = self.streams.remove(&frame.cid) {
                                stream.close();
                                sendch.send(frame).unwrap();
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
        log::debug!("TunnelStub read worker stopped");
    }

    fn write_worker(sendch: crossbeam_channel::Receiver<Frame>, closech: crossbeam_channel::Receiver<()>) {
        log::debug!("TunnelStub write worker started");
        loop {
            select!(
                recv(sendch) -> ref frame => {
                    if let Err(err) = self.send_frame(frame) {
                        log::error!("Failed to send frame: {:?}", err);
                        break;
                    }
                },
                recv(closech.clone()) -> _ => {
                    log::debug!("TunnelStub write worker stopping due to close signal");
                    break;
                }
            );
        }
        log::debug!("TunnelStub write worker stopped");
    }

    fn send_frame(&self, frame: &Frame) -> io::Result<()> {
        let frames = FrameSegment::new(frame);
        for smallframe in &frames {
            let binary_data = self.serizer.serialize(&smallframe);
            log::debug!("TunnelStub send frame: {} {} {}", smallframe.cid, smallframe.frame_type, smallframe.content_length());
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

    pub fn start_stream(&self, addr: &[u8]) -> Arc<Stream> {
        let cid = utils::generate_uuid();
        let stream = Arc::new(Stream::new(cid.clone(), addr));
        self.streams.insert(cid, stream.clone());
        let frame = Frame::new_init(cid, addr.to_vec());
        self.sendch.send(frame).unwrap();
        stream
    }

    pub fn set_ready(&self, stream: &Stream) {
        let frame = Frame::new_est(stream.cid.clone(), stream.addr.clone());
        self.sendch.send(frame).unwrap();
    }

    pub fn ping(&self) {
        let cid = frame::ID_ZERO.to_string();
        let data = [&utils::get_now_us().to_le_bytes()].concat().to_vec();
        let frame = Frame::new_ping(cid, data);
        self.sendch.send(frame).unwrap();
    }

    pub fn accept(&self) -> Option<Arc<Stream>> {
        match self.streamch.recv() {
            Ok(stream) => Some(stream),
            Err(_) => None,
        }
    }
}
