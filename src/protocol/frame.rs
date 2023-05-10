// protocol.rs
use rand::Rng;

pub const INIT_FRAME: u8 = 0;
pub const STREAM_FRAME: u8 = 1;
pub const FIN_FRAME: u8 = 2;
pub const RST_FRAME: u8 = 3;
pub const EST_FRAME: u8 = 4;
pub const PING_FRAME: u8 = 6;
pub const PONG_FRAME: u8 = 9;

pub const DATA_MAX_SIZE: usize = 1024 * 2;

// 这段 Rust 代码定义了名为 Frame 的结构体，代表协议中的一个帧(Frame)。该结构体有以下字段:
// version: 一个 u8 类型，表示协议版本号。
// cid: 一个 String 类型，表示连接标识符(Connection Identifier)。
// r#type: 一个 u8 类型，表示帧类型(Frame type)，由常量定义。
// data: 一个 Vec<u8> 类型，表示包含在帧中的数据。
// stime: 一个可选的 i32 类型，表示发送时间(stime)。
// atime: 一个可选的 i32 类型，表示到达时间(atime)。
// 这个结构体使用了 #[derive(Debug)] 属性，以便可以在调试时打印结构体的内容，方便调试。 pub 关键字用于公开所有成员字段，以便其他 crate 或模块可以访问和修改。
#[derive(Debug)]
pub struct Frame {
    pub version: u8,
    pub cid: String,
    pub r#type: u8,
    pub data: Vec<u8>,
    pub stime: Option<i32>,
    pub atime: Option<i32>,
}

impl Frame {
    pub fn new(version: u8, cid: String, r#type: u8, data: Vec<u8>) -> Frame {
        Frame {
            version,
            cid,
            r#type,
            data,
            stime: None,
            atime: None,
        }
    }

}

/**
 *
 * // required: cid, type,  data
 * @param {*} frame
 * |<-version[1]->|<--cidLen[1]-->|<---(cid)---->|<--type[1]-->|<--dataLen[2]-->|<-------data------>|
 * |-----s1 ------|-------s2------|-----s3 ------|-------s4----|-------s5 ------|--------s6---------|
 * @returns
 */

// encode encodes the given frame into binary data
pub fn encode(frame: &Frame) -> Vec<u8> {
    // get the version, CID length and data length
    let version = frame.version;
    let cid_len = frame.cid.len();
    let data_len = frame.data.len();

    // create buffers for each part of the frame
    let mut ver_buf = vec![version];
    let mut cid_len_buf = vec![];
    cid_len_buf.extend_from_slice(&(cid_len as u8).to_be_bytes());
    let cid_buf = frame.cid.as_bytes().to_vec();
    let type_buf = vec![frame.r#type];
    let mut data_len_buf = vec![];
    if data_len <= u16::MAX as usize {
        data_len_buf.extend_from_slice(&(data_len as u16).to_be_bytes());
    } else {
        panic!("Data too long!");
    }

    ver_buf.extend(cid_len_buf); //2
    ver_buf.extend(cid_buf); // 32
    ver_buf.extend(type_buf); // 1 
    ver_buf.extend(data_len_buf); // 2
    ver_buf.extend(&frame.data); //
    // concatenate all the buffers together to produce the binary data
    ver_buf
}

// decode decodes the given binary data into a frame
pub fn decode(binary_data: &[u8]) -> Result<Frame, String> {
    // check if the binary data has at least 6 bytes
    if binary_data.len() < 6 {
        return Err("Invalid binary data length1".to_string());
    }

    // extract the version, CID length, CID buffer, type, and data length from the binary data
    let version = binary_data[0];
    let cid_len = binary_data[1] as usize;
    if binary_data.len() < cid_len + 5 {
        return Err("Invalid binary data length2".to_string());
    }
    let cid_buf = String::from_utf8_lossy(&binary_data[2..cid_len + 2]).to_string();
    let r#type = binary_data[cid_len + 2];
    let data_len = (binary_data[cid_len + 3] as usize) << 8 | (binary_data[cid_len + 4] as usize);

    // check if the binary data has enough bytes to hold the specified data length
    let data_start = cid_len + 5;
    println!("data_start:{}, data_len:{}",data_start,data_len);
    if binary_data.len() < data_start + data_len {
        return Err("Invalid binary data length3".to_string());
    }

    
    // extract the data buffer from the binary data
    let data = binary_data[data_start..data_start + data_len].to_vec();

    // create a new frame with the extracted information
    let mut frame = Frame::new(version, cid_buf, r#type, data);

    // update the stime and atime fields based on the type of frame
    if r#type == PING_FRAME {
        let stime_str = String::from_utf8_lossy(&frame.data[..13]).to_string();
        frame.stime = stime_str.parse().ok();
    } else if r#type == PONG_FRAME {
        let stime_str = String::from_utf8_lossy(&frame.data[..13]).to_string();
        frame.stime = stime_str.parse().ok();
        let atime_str = String::from_utf8_lossy(&frame.data[13..26]).to_string();
        frame.atime = atime_str.parse().ok();
    }

    Ok(frame)
}

pub fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..length).map(|_| rng.gen()).collect();
    bytes
}


pub fn split_frame(frame1: &Frame) -> Vec<Frame> {
    let mut frames = Vec::new();
    let length: usize = frame1.data.len();
    if length <= DATA_MAX_SIZE {
        frames.push(Frame::new(
            frame1.version,
            frame1.cid.clone(),
            frame1.r#type,
            frame1.data.clone(),
        ));
    } else {
        let mut offset = 0;
        while offset < length {
            let end_index = std::cmp::min(offset + DATA_MAX_SIZE, length);
            let segment = frame1.data[offset..end_index].to_vec();
            frames.push(Frame::new(
                frame1.version,
                frame1.cid.clone(),
                frame1.r#type,
                segment,
            ));
            offset = end_index;
        }
    }
    frames
}