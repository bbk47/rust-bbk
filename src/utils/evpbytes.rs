use md5::{Digest, Md5};

pub fn evp_bytes_to_key(password: &str, key_len: usize, iv_len: usize) -> (Vec<u8>, Vec<u8>) {
    const MD5_LEN: usize = 16;
    let total = key_len + iv_len;

    let mut ret = vec![0u8; total];
    let pass_byte = password.as_bytes();

    let mut last_md5: Option<[u8; MD5_LEN]> = None;
    for (i, chunk) in ret.chunks_mut(MD5_LEN).enumerate() {
        let mut digest = Md5::new();
        if let Some(prev_md5) = last_md5 {
            digest.update(&prev_md5);
        }
        digest.update(pass_byte);

        let md5 = md5sum(digest.finalize());
        last_md5 = Some(md5);

        let len = std::cmp::min(chunk.len(), total - i * MD5_LEN);
        chunk[..len].copy_from_slice(&md5[..len]);
    }

    (ret[..key_len].to_vec(), ret[key_len..].to_vec())
}

fn md5sum(data: impl AsRef<[u8]>) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher.finalize().into()
}
