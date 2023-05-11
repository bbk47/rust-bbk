use openssl::symm::{decrypt, encrypt, Cipher};
use std::error::Error;

fn aes_cfb_128_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_128_cfb128();
    let encrypted = encrypt(cipher, key, Some(iv), data)?;
    Ok(encrypted)
}

fn aes_cfb_128_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_128_cfb128();
    let decrypted = decrypt(cipher, key, Some(iv), data)?;
    Ok(decrypted)
}

fn aes_cfb_192_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_192_cfb128();
    let encrypted = encrypt(cipher, key, Some(iv), data)?;
    Ok(encrypted)
}

fn aes_cfb_192_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_192_cfb128();
    let decrypted = decrypt(cipher, key, Some(iv), data)?;
    Ok(decrypted)
}

fn aes_cfb_256_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_256_cfb128();
    let encrypted = encrypt(cipher, key, Some(iv), data)?;
    Ok(encrypted)
}

fn aes_cfb_256_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_256_cfb128();
    let decrypted = decrypt(cipher, key, Some(iv), data)?;
    Ok(decrypted)
}

fn aes_cbc_128_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_128_cbc();
    let encrypted = encrypt(cipher, key, Some(iv), data)?;
    Ok(encrypted)
}

fn aes_cbc_128_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_128_cbc();
    let decrypted = decrypt(cipher, key, Some(iv), data)?;
    Ok(decrypted)
}

fn aes_cbc_192_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_192_cbc();
    let encrypted = encrypt(cipher, key, Some(iv), data)?;
    Ok(encrypted)
}

fn aes_cbc_192_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_192_cbc();
    let decrypted = decrypt(cipher, key, Some(iv), data)?;
    Ok(decrypted)
}

fn aes_cbc_256_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_256_cbc();
    let encrypted = encrypt(cipher, key, Some(iv), data)?;
    Ok(encrypted)
}

fn aes_cbc_256_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher = Cipher::aes_256_cbc();
    let decrypted = decrypt(cipher, key, Some(iv), data)?;
    Ok(decrypted)
}




