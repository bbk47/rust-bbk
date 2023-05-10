#[path = "../src/utils/encrypt.rs"]
mod encrypt;

use encrypt::Encryptor;

fn test_encrypt_decrypt1(method: &str, password: &str, input: &[u8]) {
    let mut encryptor = Encryptor::new(method, password);
    let mut ciphertext = input.to_vec();
    encryptor.encrypt(&mut ciphertext);

    let mut decryptor = Encryptor::new(method, password);
    let mut plaintext = ciphertext.clone();
    decryptor.decrypt(&mut plaintext);

    assert_eq!(input, plaintext.as_slice());
}

#[test]
fn test_encrypt_decrypt() {
    let methods = &["aes-128-cfb", "aes-192-cfb", "aes-256-cfb"];

    let password = "password";
    let input = b"hello world";

    for method in methods {
        test_encrypt_decrypt1(method, password, input);
    }
}
