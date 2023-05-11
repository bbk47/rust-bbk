#[path = "../src/utils/encrypt.rs"]
mod encrypt;

use encrypt::Encryptor;



#[test]
fn test_encrypt_decrypt() {
    let methods = &["aes-128-cfb", "aes-192-cfb", "aes-256-cfb","aes-128-cbc", "aes-192-cbc", "aes-256-cbc","aes-128-ctr", "aes-192-ctr", "aes-256-ctr"];

    let password = "password";
    let input = b"hello world";

    for method in methods {
        let mut encryptor = Encryptor::new(method, password);
        let mut ciphertext = input.to_vec();
        encryptor.encrypt(&mut ciphertext);
    
        let mut decryptor = Encryptor::new(method, password);
        let mut plaintext = ciphertext.clone();
        decryptor.decrypt(&mut plaintext);
    
        assert_eq!(input, plaintext.as_slice());
    }
}
