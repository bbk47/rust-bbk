#[path = "../src/utils/evpbytes.rs"]
mod evpbytes;

use hex;

#[test]
fn generate_key_iv() {
    let (key, iv) = evpbytes::evp_bytes_to_key("password", 16, 16);
    println!("key:{},iv:{}",hex::encode(&key),hex::encode(&iv));
    assert_eq!(16, key.len());
    assert_eq!(16, iv.len());

    let (key, iv) = evpbytes::evp_bytes_to_key("password", 16, 32);
    println!("key:{},iv:{}",hex::encode(&key),hex::encode(&iv));
    assert_eq!(16, key.len());
    assert_eq!(32, iv.len());
}
