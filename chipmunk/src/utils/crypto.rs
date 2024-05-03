use openssl::symm::Cipher;
use rand::Rng;

pub fn encrypt(data: &str, key: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    // aes_256_cbc require 32 byte key and
    // aes_128_cbc require 16 byte key
    match key.as_bytes().len() {
        len if len < 32 => anyhow::bail!("Invalid key length {len}; expected 32. AES256 require a 32 byte (256 bit) key"),
        len if len > 32 => log::warn!("Key size is larger than 32 bytes. Using the first 32 bytes for encryption and discarding the rest"),
        _ => (), // all good
    }

    // Trim the key
    let key_bytes = &key.as_bytes()[..32];

    // Generate a random IV
    let mut iv = [0u8; 16];
    rand::thread_rng().fill(&mut iv);

    let cipher = Cipher::aes_256_cbc();
    match openssl::symm::encrypt(cipher, key_bytes, Some(&iv), data.as_bytes()) {
        Ok(v) => Ok((v, iv.to_vec())),
        Err(e) => anyhow::bail!("Error encrypting: {}", e),
    }
}

pub fn decrypt(data: &[u8], key: &str, iv: &[u8]) -> anyhow::Result<String> {
    // aes_256_cbc require 32 byte key and
    // aes_128_cbc require 16 byte key
    match key.as_bytes().len() {
        len if len < 32 => anyhow::bail!("Invalid key length {len}; expected 32. AES256 require a 32 byte (256 bit) key"),
        len if len > 32 => log::warn!("Key size is larger than 32 bytes. Using the first 32 bytes for encryption and discarding the rest"),
        _ => (), // all good
    }

    // Trim the key
    let key_bytes = &key.as_bytes()[..32];

    if iv.len() != 16 {
        anyhow::bail!(
            "Invalid IV size {}; expected 16. AES256 require a 16 byte (128 bit) iv",
            key_bytes.len()
        );
    }

    let cipher = Cipher::aes_256_cbc();
    match openssl::symm::decrypt(cipher, key_bytes, Some(iv), data) {
        Ok(v) => Ok(std::str::from_utf8(&v)?.to_string()),
        Err(e) => anyhow::bail!("Error decrypting, please check the encryption key: {}", e),
    }
}

#[test]
#[ignore = "This test will modify auth key entries in production database"]
fn test_encryption() {
    let plaintext = "Hello world!";
    let key = "0123456789abcdef0123456789abcdef";

    let (encrypted_data, iv) = encrypt(plaintext, key).expect("Error encrypting");
    let decrypted_data = decrypt(&encrypted_data, key, &iv).expect("Error decrypting");

    assert_eq!(decrypted_data, plaintext);
}
