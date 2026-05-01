use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{anyhow, bail, Context, Result};
use argon2::Argon2;
use rand::{rngs::OsRng, RngCore};
use std::fs;
use zeroize::Zeroize;

const MAGIC: &[u8; 9] = b"ECHOSAFE1";
const VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const HEADER_LEN: usize = MAGIC.len() + 1 + SALT_LEN + NONCE_LEN;

pub fn encrypt_file_to_path(
    input_path: &str,
    output_path: &str,
    password: &mut String,
) -> Result<()> {
    let mut plaintext =
        fs::read(input_path).with_context(|| format!("Failed to read input file: {input_path}"))?;
    let encrypted = encrypt_bytes(&plaintext, password.as_bytes())?;
    plaintext.zeroize();
    password.zeroize();
    fs::write(output_path, encrypted)
        .with_context(|| format!("Failed to write encrypted file: {output_path}"))?;
    Ok(())
}

pub fn decrypt_file_to_path(
    input_path: &str,
    output_path: &str,
    password: &mut String,
) -> Result<()> {
    let encrypted = fs::read(input_path)
        .with_context(|| format!("Failed to read encrypted file: {input_path}"))?;
    let mut plaintext = decrypt_bytes(&encrypted, password.as_bytes())?;
    password.zeroize();
    fs::write(output_path, &plaintext)
        .with_context(|| format!("Failed to write decrypted file: {output_path}"))?;
    plaintext.zeroize();
    Ok(())
}

fn encrypt_bytes(plaintext: &[u8], password: &[u8]) -> Result<Vec<u8>> {
    if password.is_empty() {
        bail!("Password cannot be empty");
    }

    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);

    let mut key = derive_key(password, &salt)?;
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow!("Failed to initialize AES-256-GCM"))?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| anyhow!("Encryption failed"))?;

    let mut package = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    package.extend_from_slice(MAGIC);
    package.push(VERSION);
    package.extend_from_slice(&salt);
    package.extend_from_slice(&nonce_bytes);
    package.extend_from_slice(&ciphertext);

    key.zeroize();
    salt.zeroize();
    nonce_bytes.zeroize();

    Ok(package)
}

fn decrypt_bytes(encrypted: &[u8], password: &[u8]) -> Result<Vec<u8>> {
    if password.is_empty() {
        bail!("Password cannot be empty");
    }

    if encrypted.len() < HEADER_LEN {
        bail!("Encrypted file is too short");
    }

    if &encrypted[..MAGIC.len()] != MAGIC {
        bail!("Invalid EchoSafe encrypted file format");
    }

    let version = encrypted[MAGIC.len()];
    if version != VERSION {
        bail!("Unsupported EchoSafe encrypted file version: {version}");
    }

    let salt_start = MAGIC.len() + 1;
    let nonce_start = salt_start + SALT_LEN;
    let ciphertext_start = nonce_start + NONCE_LEN;

    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&encrypted[salt_start..nonce_start]);
    let nonce = &encrypted[nonce_start..ciphertext_start];
    let ciphertext = &encrypted[ciphertext_start..];

    let mut key = derive_key(password, &salt)?;
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow!("Failed to initialize AES-256-GCM"))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| anyhow!("Invalid password or corrupted encrypted file"))?;

    key.zeroize();
    salt.zeroize();

    Ok(plaintext)
}

fn derive_key(password: &[u8], salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let mut key = [0u8; KEY_LEN];
    Argon2::default()
        .hash_password_into(password, salt, &mut key)
        .map_err(|_| anyhow!("Failed to derive encryption key"))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::{decrypt_bytes, encrypt_bytes};

    #[test]
    fn round_trip_encrypt_decrypt() {
        let plaintext = b"wave bytes go here";
        let password = b"class-project-demo";

        let encrypted = encrypt_bytes(plaintext, password).expect("encryption should succeed");
        let decrypted = decrypt_bytes(&encrypted, password).expect("decryption should succeed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn rejects_wrong_password() {
        let encrypted =
            encrypt_bytes(b"secret audio", b"correct-password").expect("encryption should succeed");
        let result = decrypt_bytes(&encrypted, b"wrong-password");

        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_magic() {
        let result = decrypt_bytes(b"not-an-echosafe-file", b"password");

        assert!(result.is_err());
    }
}
