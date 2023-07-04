

mod worker;
mod steam;

pub use worker::TunnelStub;
// pub use steam::relay;
pub use steam::VirtualStream;



// match frame.r#type {
//     FrameType::Ping => {
//         let now = Instant::now();
//         let duration = now.duration_since(last_ping).as_millis() as i64;
//         last_ping = now;

//         let data = [&frame.content(), &utils::i64_to_bytes_le(duration)]
//         .concat()
//         .to_vec();
//         let pong = Frame::new_pong(cid.clone(), data);
//         sender_send.send(pong).unwrap();
//     }
//     FrameType::Pong => {
//         let (up, down) = match utils::parse_ping_pong(&frame.content()) {
//             Ok((up, down)) => (up, down),
//             Err(_) => continue,
//         };
//         if let Some(ref mut handler) = &mut self.pong_func {
//             handler(up, down);
//         }
//     }
//     FrameType::Init => {
//         let stream = Arc::new(CopyStream::new(frame.cid.clone(), &frame.content()));
//         self.streams.insert(frame.cid.clone(), stream.clone());
//         sender_send.send(frame).unwrap();
//         self.streamch.send(stream).unwrap();
//     }
//     FrameType::CopyStream => {
//         if let Some(stream) = self.streams.get(&frame.cid) {
//             stream.produce(&frame.content());
//         }
//     }
//     FrameType::Fin | FrameType::Rst => {
//         if let Some(stream) = self.streams.remove(&frame.cid) {
//             stream.close();
//             sender_send.send(frame).unwrap();
//         }
//     }
//     FrameType::Est => {
//             if let Some(stream) = self.streams.get(&frame.cid) {
//                 self.streamch.send(stream.clone()).unwrap();
//             }
//     }
//     _ => {
//             log::warn!("Unexpected frame type: {}", frame.frame_type);
//     }
// }