use openssl::symm::{decrypt, encrypt, Cipher};
use std::error::Error;

mod evpbytes;

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
                let  ( key, iv) = evpbytes::evp_bytes_to_key(&password,key_len,iv_len);
                $name { key, iv }
            }
        }

        impl EncryptorImpl for $name {
            fn encrypt(&self, data: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
                let mut encrypted = encrypt($cipher, &self.key, Some(&self.iv), data)?;
                // Add padding to match the behavior of OpenSSL's EVP_*_encrypt functions
                let block_size = $cipher.block_size();
                let pad_len = block_size - (encrypted.len() % block_size);
                encrypted.extend(vec![pad_len as u8; pad_len]);
                *data = encrypted;
                Ok(())
            }

            fn decrypt(&self, data: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
                let mut decrypted = decrypt($cipher, &self.key, Some(&self.iv), data)?;
                // Remove padding added during encryption
                let pad_len = decrypted.last().cloned().unwrap_or(0) as usize;
                if pad_len >= $cipher.block_size() || decrypted.len() < pad_len {
                    return Err("Invalid padding length".into());
                }
                decrypted.truncate(decrypted.len() - pad_len);
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