use openssl::symm::{decrypt, encrypt, Cipher};
use std::error::Error;


macro_rules! impl_aes_encryptor {
    ($name:ident, $cipher:expr) => {
        struct $name {
            key: Vec<u8>,
            iv: Vec<u8>,
        }

        impl $name {
            fn new(password: &str) -> Self {
                let key_len = $cipher.key_len();
                let iv_len = $cipher.iv_len().unwrap();
                let (key, iv) = evp_bytes_to_key(&password, key_len, iv_len);
                println!("key:{},iv:{}", hex::encode(&key), hex::encode(&iv));
                $name { key, iv }
            }
        }

        impl EncryptorImpl for $name {
            fn encrypt(&self, data: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
                let encrypted = encrypt($cipher, &self.key, Some(&self.iv), data)?;
                // Add padding to match the behavior of OpenSSL's EVP_*_encrypt functions
                // let block_size = $cipher.block_size();
                // let pad_len = block_size - (encrypted.len() % block_size);
                // encrypted.extend(vec![pad_len as u8; pad_len]);
                *data = encrypted;
                Ok(())
            }

            fn decrypt(&self, data: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
                let decrypted = decrypt($cipher, &self.key, Some(&self.iv), data)?;
                // Remove padding added during encryption
                // let pad_len = decrypted.last().cloned().unwrap_or(0) as usize;
                // if pad_len >= $cipher.block_size() || decrypted.len() < pad_len {
                //     return Err("Invalid padding length".into());
                // }
                // decrypted.truncate(decrypted.len() - pad_len);
                *data = decrypted;
                Ok(())
            }
        }
    };
}

impl_aes_encryptor!(Aes128CfbEncryptor, Cipher::aes_128_cfb128());
impl_aes_encryptor!(Aes192CfbEncryptor, Cipher::aes_192_cfb128());
impl_aes_encryptor!(Aes256CfbEncryptor, Cipher::aes_256_cfb128());
impl_aes_encryptor!(Aes128CbcEncryptor, Cipher::aes_128_cbc());
impl_aes_encryptor!(Aes192CbcEncryptor, Cipher::aes_192_cbc());
impl_aes_encryptor!(Aes256CbcEncryptor, Cipher::aes_256_cbc());
impl_aes_encryptor!(Aes128CtrEncryptor, Cipher::aes_128_ctr());
impl_aes_encryptor!(Aes192CtrEncryptor, Cipher::aes_192_ctr());
impl_aes_encryptor!(Aes256CtrEncryptor, Cipher::aes_256_ctr());

pub struct Encryptor {
    impl_: Box<dyn EncryptorImpl>,
}

impl Encryptor {
    pub fn new(method: &str, password: &str) -> Self {
        let impl_: Box<dyn EncryptorImpl> = match method {
            "aes-128-cfb" => Box::new(Aes128CfbEncryptor::new(password)),
            "aes-192-cfb" => Box::new(Aes192CfbEncryptor::new(password)),
            "aes-256-cfb" => Box::new(Aes256CfbEncryptor::new(password)),
            "aes-128-cbc" => Box::new(Aes128CbcEncryptor::new(password)),
            "aes-192-cbc" => Box::new(Aes192CbcEncryptor::new(password)),
            "aes-256-cbc" => Box::new(Aes256CbcEncryptor::new(password)),
            "aes-128-ctr" => Box::new(Aes128CtrEncryptor::new(password)),
            "aes-192-ctr" => Box::new(Aes192CtrEncryptor::new(password)),
            "aes-256-ctr" => Box::new(Aes256CtrEncryptor::new(password)),
            _ => unimplemented!(),
        };
        Encryptor { impl_ }
    }

    pub fn encrypt(&self, data: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
        self.impl_.encrypt(data)
    }

    pub fn decrypt(&self, data: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
        self.impl_.decrypt(data)
    }
}

trait EncryptorImpl {
    fn encrypt(&self, data: &mut Vec<u8>) -> Result<(), Box<dyn Error>>;
    fn decrypt(&self, data: &mut Vec<u8>) -> Result<(), Box<dyn Error>>;
}


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
    // let mut temp_buf = vec![0; md5_len + pass_byte.len()];

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
