#[path = "../src/utils/encrypt.rs"]
mod encrypt;

use encrypt::Encryptor;

// 5f4dcc3b5aa765d61d8327deb882cf99
// 2b95990a9151374abd8ff8c5a7a0fe08
// 7360f22a5a1a4b2e7978d9

#[test]
fn test_encrypt_decrypt() {
    let methods = &[
        "aes-128-cfb",
        "aes-192-cfb",
        "aes-256-cfb",
        "aes-128-cbc",
        "aes-192-cbc",
        "aes-256-cbc",
        "aes-128-ctr",
        "aes-192-ctr",
        "aes-256-ctr",
    ];

    let password = "password";
    let input = b"hello world";

    for method in methods {
        let  encryptor = Encryptor::new(method, password);
        let mut ciphertext = input.to_vec();
        encryptor.encrypt(&mut ciphertext).unwrap();
        println!("method:{},ciphertext:{}", method, hex::encode(&ciphertext));

        let  decryptor = Encryptor::new(method, password);
        let mut plaintext = ciphertext.clone();
        decryptor.decrypt(&mut plaintext).unwrap();

        assert_eq!(input, plaintext.as_slice());
    }
}
