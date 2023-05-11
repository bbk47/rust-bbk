use openssl::hash::{Hasher, MessageDigest};
use std::collections::VecDeque;

fn md5sum(d: &[u8]) -> Vec<u8> {
    let mut hasher = Hasher::new(MessageDigest::md5()).unwrap();
    hasher.update(d).unwrap();
    hasher.finish().unwrap().to_vec()
}

pub fn evp_bytes_to_key(password: &str, key_len: usize, iv_len: usize) -> (Vec<u8>, Vec<u8>) {
    let md5_len = 16;
    let total = key_len + iv_len;
    let mut ret = vec![0; total];
    let pass_byte = password.as_bytes();
    let mut temp_buf = vec![0; md5_len + pass_byte.len()];

    let mut last = md5sum(pass_byte);
    let mut offset = 0;
    while offset < total {
        if offset == 0 {
            last = md5sum(pass_byte);
        } else {
            let mut deque = VecDeque::with_capacity(last.len() + pass_byte.len());
            deque.extend(last.iter());
            deque.extend(pass_byte.iter());
            let concatenated = deque.into_iter().collect::<Vec<u8>>();
            last = md5sum(&concatenated);
        }
        let len = std::cmp::min(md5_len, total - offset);
        ret[offset..offset + len].copy_from_slice(&last[..len]);
        offset += md5_len;
    }
    (ret[..key_len].to_vec(), ret[key_len..].to_vec())
}
