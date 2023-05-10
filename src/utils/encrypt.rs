use aes::{Aes128, Aes192, Aes256, BlockCipher, NewBlockCipher};
use aes::{BlockCipher, NewBlockCipher};

use cipher::{NewStreamCipher, StreamCipher};

use md5::{Md5, Digest};

fn evp_bytes_to_key(password: &str, key_len: usize, iv_len: usize) -> (Vec<u8>, Vec<u8>) {
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

        let md5 = md5sum(digest.finalize().as_slice());
        last_md5 = Some(md5);

        let len = std::cmp::min(chunk.len(), total - i * MD5_LEN);
        chunk[..len].copy_from_slice(&md5[..len]);
    }

    (ret[..key_len].to_vec(), ret[key_len..].to_vec())
}

fn md5sum(data: &[u8]) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher.finalize().into()
}
pub struct Encryptor {
    enc: Box<dyn StreamCipher>,
    dec: Box<dyn StreamCipher>,
}

 impl Encryptor {
    pub fn new(method: &str, password: &str) -> Self {
        let key_len = match method {
            "aes-128-cfb" => 16,
            "aes-192-cfb" => 24,
            "aes-256-cfb" => 32,
            "aes-128-ctr" => 16,
            "aes-192-ctr" => 24,
            "aes-256-ctr" => 32,
            _ => panic!("Unsupported encryption method"),
        };

        let iv_len = 16;
        let (key, iv) = evp_bytes_to_key(password, key_len, iv_len);

        let enc: Box<_> = match method {
            "aes-128-cfb" => aes_cfb::<Aes128>(&key, &iv),
            "aes-192-cfb" => aes_cfb::<Aes192>(&key, &iv),
            "aes-256-cfb" => aes_cfb::<Aes256>(&key, &iv),
            _ => panic!("Unsupported encryption method"),
        };

        let dec = enc.clone();

        Self { enc, dec }
    }

    pub fn encrypt(&mut self, buf: &mut [u8]) {
        self.enc.apply_keystream(buf);
    }

    pub fn decrypt(&mut self, buf: &mut [u8]) {
        self.dec.apply_keystream(buf);
    }
}

fn aes_cfb<C: BlockCipher + NewBlockCipher>(key: &[u8], iv: &[u8]) -> Box<dyn StreamCipher> {
    let cipher = C::new_var(key).unwrap();
    Box::new(cipher.encrypt_cfb8(iv))
}