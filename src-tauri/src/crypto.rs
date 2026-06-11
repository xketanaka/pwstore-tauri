use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;
use scrypt::{scrypt, Params};

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

// N=16384 (log_n=14), r=8, p=1 — Node.js crypto.scryptSync のデフォルトと同じ
const SCRYPT_LOG_N: u8 = 14;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let mut key = [0u8; 32];
    let params = Params::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P, 32).map_err(|e| e.to_string())?;
    scrypt(passphrase.as_bytes(), salt, &params, &mut key).map_err(|e| e.to_string())?;
    Ok(key)
}

/// JSON バイト列を暗号化し、salt + nonce + ciphertext を返す
pub fn encrypt(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let key_bytes = derive_key(passphrase, &salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// salt + nonce + ciphertext を復号し、plaintext を返す
pub fn decrypt(data: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    if data.len() < SALT_LEN + NONCE_LEN {
        return Err("データが短すぎます".to_string());
    }
    let (salt, rest) = data.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);

    let key_bytes = derive_key(passphrase, salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "復号に失敗しました。パスフレーズが違うか、データが壊れています。".to_string())
}
