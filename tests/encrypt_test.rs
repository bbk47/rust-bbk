use bbk::utils::encrypt::{evp_bytes_to_key, Encryptor};

const METHODS: &[&str] = &[
    "aes-128-cfb",
    "aes-192-cfb",
    "aes-256-cfb",
    "aes-128-ctr",
    "aes-192-ctr",
    "aes-256-ctr",
    "rc4-md5",
    "rc4-md5-6",
];

/// Encrypting on one side and decrypting on the other (with the IVs in the
/// roles SecureConn uses) must round-trip for every supported method, across
/// several successive chunks (the keystream is continuous).
#[test]
fn enc_dec_roundtrip_all_methods() {
    for method in METHODS {
        let enc = Encryptor::new(method, "p@ssword").unwrap();
        let iv = vec![0x11u8; enc.iv_len()];
        let mut e = enc.new_enc_stream(&iv).unwrap();
        let mut d = enc.new_dec_stream(&iv).unwrap();

        for chunk in [&b"hello world"[..], &b""[..], &[0u8; 4096][..], &b"trailing"[..]] {
            let ct = e.xor(chunk).unwrap();
            assert_eq!(ct.len(), chunk.len(), "method={method} length preserved");
            let pt = d.xor(&ct).unwrap();
            assert_eq!(pt, chunk, "method={method} roundtrip");
        }
    }
}

/// A continuous stream cipher must produce different ciphertext for the same
/// plaintext at different stream positions (i.e. it is not ECB-like).
#[test]
fn keystream_is_position_dependent() {
    let enc = Encryptor::new("aes-256-cfb", "p@ssword").unwrap();
    let iv = vec![0x22u8; enc.iv_len()];
    let mut e = enc.new_enc_stream(&iv).unwrap();
    let first = e.xor(b"AAAAAAAA").unwrap();
    let second = e.xor(b"AAAAAAAA").unwrap();
    assert_ne!(first, second);
}

/// Two independent encryptors built from the same method+password must agree,
/// so a Rust client and a Rust/Go server derive identical key material.
#[test]
fn independent_encryptors_interop() {
    let iv = vec![0x33u8; 16];
    let client = Encryptor::new("aes-128-ctr", "secret").unwrap();
    let server = Encryptor::new("aes-128-ctr", "secret").unwrap();
    let mut ce = client.new_enc_stream(&iv).unwrap();
    let mut sd = server.new_dec_stream(&iv).unwrap();
    let ct = ce.xor(b"cross-impl").unwrap();
    assert_eq!(sd.xor(&ct).unwrap(), b"cross-impl");
}

#[test]
fn unsupported_method_errors() {
    assert!(Encryptor::new("chacha20", "x").is_err());
}

/// EVP_BytesToKey(MD5) reference vector. For password "password" the first MD5
/// block is md5("password") = 5f4dcc3b5aa765d61d8327deb882cf99; a 16-byte key
/// equals exactly that block.
#[test]
fn evp_bytes_to_key_known_vector() {
    let (key, _iv) = evp_bytes_to_key("password", 16, 0);
    assert_eq!(hex::encode(&key), "5f4dcc3b5aa765d61d8327deb882cf99");
}

/// For key_len+iv_len > 16 the derivation chains MD5(prev || password).
#[test]
fn evp_bytes_to_key_chained_length() {
    let (key, iv) = evp_bytes_to_key("password", 32, 16);
    assert_eq!(key.len(), 32);
    assert_eq!(iv.len(), 16);
    // First 16 key bytes are still md5("password").
    assert_eq!(hex::encode(&key[..16]), "5f4dcc3b5aa765d61d8327deb882cf99");
}
