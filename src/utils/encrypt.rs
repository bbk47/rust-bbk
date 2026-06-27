use openssl::hash::{Hasher, MessageDigest};
use openssl::symm::{Cipher, Crypter, Mode};

/// Cipher family selected by the configured `method`. Mirrors the methods
/// supported by the Go `bbk47/toolbox` package so the wire format matches.
#[derive(Clone, Copy)]
enum Kind {
    /// AES style cipher that takes the connection IV directly.
    Iv,
    /// rc4-md5 family: stream key = md5(evp_key || connection_iv), no IV on the cipher.
    Rc4Md5,
}

/// Holds the password-derived key material plus the cipher selection. One
/// `Encryptor` is created per tunnel; each connection then derives fresh
/// per-connection enc/dec streams from random IVs (see `tunnel::SecureConn`).
#[derive(Clone)]
pub struct Encryptor {
    kind: Kind,
    cipher: Cipher,
    key: Vec<u8>,
    iv_len: usize,
}

impl Encryptor {
    pub fn new(method: &str, password: &str) -> Result<Self, String> {
        let (kind, cipher, key_len, iv_len): (Kind, Cipher, usize, usize) = match method {
            "aes-128-cfb" => (Kind::Iv, Cipher::aes_128_cfb128(), 16, 16),
            "aes-192-cfb" => (Kind::Iv, Cipher::aes_192_cfb128(), 24, 16),
            "aes-256-cfb" => (Kind::Iv, Cipher::aes_256_cfb128(), 32, 16),
            "aes-128-ctr" => (Kind::Iv, Cipher::aes_128_ctr(), 16, 16),
            "aes-192-ctr" => (Kind::Iv, Cipher::aes_192_ctr(), 24, 16),
            "aes-256-ctr" => (Kind::Iv, Cipher::aes_256_ctr(), 32, 16),
            "rc4-md5" => (Kind::Rc4Md5, Cipher::rc4(), 16, 16),
            "rc4-md5-6" => (Kind::Rc4Md5, Cipher::rc4(), 16, 6),
            _ => return Err(format!("unsupported method: {}", method)),
        };
        let (key, _iv) = evp_bytes_to_key(password, key_len, iv_len);
        Ok(Encryptor { kind, cipher, key, iv_len })
    }

    /// Length of the random IV exchanged at the start of each connection.
    pub fn iv_len(&self) -> usize {
        self.iv_len
    }

    pub fn new_enc_stream(&self, iv: &[u8]) -> Result<StreamCrypter, String> {
        self.new_stream(Mode::Encrypt, iv)
    }

    pub fn new_dec_stream(&self, iv: &[u8]) -> Result<StreamCrypter, String> {
        self.new_stream(Mode::Decrypt, iv)
    }

    fn new_stream(&self, mode: Mode, iv: &[u8]) -> Result<StreamCrypter, String> {
        let inner = match self.kind {
            Kind::Iv => {
                let crypter = Crypter::new(self.cipher, mode, &self.key, Some(iv)).map_err(|e| e.to_string())?;
                Inner::Ossl { crypter, block_size: self.cipher.block_size().max(1) }
            }
            // rc4-md5: stream key = md5(evp_key || connection_iv). RC4 lives in
            // OpenSSL 3's legacy provider (often unavailable with a static build),
            // so use a self-contained RC4 to stay wire-compatible with Go.
            Kind::Rc4Md5 => {
                let mut material = self.key.clone();
                material.extend_from_slice(iv);
                Inner::Rc4(Rc4::new(&md5sum(&material)))
            }
        };
        Ok(StreamCrypter { inner })
    }
}

/// A continuous stream cipher. Successive `xor` calls keep advancing the same
/// keystream, exactly like Go's `cipher.Stream.XORKeyStream`.
pub struct StreamCrypter {
    inner: Inner,
}

enum Inner {
    Ossl { crypter: Crypter, block_size: usize },
    Rc4(Rc4),
}

impl StreamCrypter {
    /// Transforms `input` and returns the result. Length is preserved for the
    /// stream ciphers we use (CFB/CTR/RC4 emit one output byte per input byte).
    pub fn xor(&mut self, input: &[u8]) -> Result<Vec<u8>, String> {
        match &mut self.inner {
            Inner::Ossl { crypter, block_size } => {
                let mut out = vec![0u8; input.len() + *block_size];
                let n = crypter.update(input, &mut out).map_err(|e| e.to_string())?;
                out.truncate(n);
                Ok(out)
            }
            Inner::Rc4(rc4) => Ok(rc4.apply(input)),
        }
    }
}

/// Standard RC4 stream cipher (matches Go's `crypto/rc4`). Symmetric: the same
/// routine encrypts and decrypts.
struct Rc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    fn new(key: &[u8]) -> Self {
        let mut s = [0u8; 256];
        for (i, b) in s.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut j = 0u8;
        for i in 0..256 {
            j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
            s.swap(i, j as usize);
        }
        Rc4 { s, i: 0, j: 0 }
    }

    fn apply(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(input.len());
        for &b in input {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);
            let k = self.s[(self.s[self.i as usize].wrapping_add(self.s[self.j as usize])) as usize];
            out.push(b ^ k);
        }
        out
    }
}

fn md5sum(d: &[u8]) -> Vec<u8> {
    let mut hasher = Hasher::new(MessageDigest::md5()).unwrap();
    hasher.update(d).unwrap();
    hasher.finish().unwrap().to_vec()
}

/// OpenSSL-compatible EVP_BytesToKey using MD5 (matches Go toolbox key derivation).
pub fn evp_bytes_to_key(password: &str, key_len: usize, iv_len: usize) -> (Vec<u8>, Vec<u8>) {
    let md5_len = 16;
    let total = key_len + iv_len;
    let pass_byte = password.as_bytes();

    let mut ret: Vec<u8> = Vec::with_capacity(total + md5_len);
    let mut last: Vec<u8> = Vec::new();
    while ret.len() < total {
        let mut input = Vec::with_capacity(last.len() + pass_byte.len());
        input.extend_from_slice(&last);
        input.extend_from_slice(pass_byte);
        last = md5sum(&input);
        ret.extend_from_slice(&last);
    }
    ret.truncate(total);
    (ret[..key_len].to_vec(), ret[key_len..].to_vec())
}
