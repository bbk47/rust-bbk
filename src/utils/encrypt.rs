use aes::{Aes128, Aes192, Aes256};
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cfb};
use md5::{Digest, Md5};

type Aes128Cfb = Cfb<Aes128, Pkcs7>;
type Aes192Cfb = Cfb<Aes192, Pkcs7>;
type Aes256Cfb = Cfb<Aes256, Pkcs7>;

pub struct Encryptor {
    enc: Box<dyn BlockMode>,
    dec: Box<dyn BlockMode>,
}

impl Encryptor {
    pub fn new(method: &str, password: &str) -> Result<Self, &'static str> {
        let (key_len, cipher_constructor) = match method {
            "aes-128-cfb" => (16, Aes128Cfb::new_var),
            "aes-192-cfb" => (24, Aes192Cfb::new_var),
            "aes-256-cfb" => (32, Aes256Cfb::new_var),
            _ => return Err("Unsupported encryption method"),
        };

        let iv_len = 16;
        let (key, iv) = evp_bytes_to_key(password, key_len, iv_len);

        if key.len() != key_len || iv.len() != iv_len {
            return Err("Invalid key or iv length");
        }

        let enc = cipher_constructor(&key.into(), &iv.into()).unwrap();
        let dec = Box::new(enc.clone()) as Box<dyn BlockMode>;

        Ok(Self { enc: Box::new(enc), dec })
    }

    pub fn encrypt(&self, input: &mut [u8]) -> Vec<u8> {
        let mut output = vec![0; input.len()];
        (*self.enc).encrypt(input, &mut output).expect("Encryption error");
        output
    }

    pub fn decrypt(&self, input: &mut [u8]) -> Vec<u8> {
        let mut output = vec![0; input.len()];
        (*self.dec).decrypt(input).expect("Decryption error");
        output
    }
}

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
