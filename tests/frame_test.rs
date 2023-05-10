#[path = "../src/protocol/frame.rs"]
mod protocol;

use hex;
use protocol::{Frame, encode, decode, generate_random_bytes};

#[test]
fn test_frame_static() {
    let frame1 = Frame::new (1, String::from("79d309c9e17b44fc9e1425ed5fe92d31"),  1,  vec![0x1, 0x2, 0x3, 0x4] );
    let result = encode(&frame1);
    println!("{}", result.len());
    println!("{}", hex::encode(&result));
    if result.len() != 5 + 32 + 4 {
        panic!("Test failed! Expected length is 41!");
    }
    let frame2 = decode(&result).unwrap();
    if frame2.cid != frame1.cid || frame2.r#type != frame1.r#type || hex::encode(&frame2.data) != hex::encode(&frame1.data) {
        panic!("Test failed!");
    }
}

#[test]
fn test_frame_type() {
    let frame1 = Frame::new (1, String::from("79d309c9e17b44fc9e1425ed5fe92d32"),  2,  vec![0x1, 0x2, 0x3, 0x4] );
    let result = encode(&frame1);

    let frame2 = decode(&result).unwrap();
    if frame2.cid != frame1.cid || frame2.r#type != frame1.r#type {
        panic!("Test failed!");
    }
}

#[test]
fn test_frame_dynamic_data() {
    let randata = generate_random_bytes(20);
    let frame1 = Frame::new(1,String::from("79d309c9e17b44fc9e1425ed5fe92d32"),1,randata.clone());
    let result = encode(&frame1);
    if result.len() != 5 + 32 + 20 {
        panic!("Test failed! Expected length is 57!");
    }

    let frame2 = decode(&result).unwrap();
    if frame2.cid != frame1.cid || frame2.r#type != frame1.r#type || hex::encode(&frame2.data) != hex::encode(&randata) {
        panic!("Test failed!");
    }
}