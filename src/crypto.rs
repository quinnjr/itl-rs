use aes::Aes128;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit, generic_array::GenericArray};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};

use crate::error::{ItlError, Result};

const AES_KEY: &[u8; 16] = b"BHUILuilfghuila3";
const BLOCK_SIZE: usize = 16;

pub fn decrypt_payload(encrypted: &[u8], max_crypt_size: u32) -> Result<Vec<u8>> {
    let payload_len = encrypted.len();
    let aligned = payload_len & !0xf;
    let crypt_size = aligned.min(max_crypt_size as usize);

    let cipher = Aes128::new(GenericArray::from_slice(AES_KEY));

    let mut buf = encrypted.to_vec();
    for chunk in buf[..crypt_size].chunks_exact_mut(BLOCK_SIZE) {
        cipher.decrypt_block(GenericArray::from_mut_slice(chunk));
    }

    let mut decompressed = Vec::new();
    let mut decoder = ZlibDecoder::new(&buf[..]);
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| ItlError::Decompression(e.to_string()))?;

    Ok(decompressed)
}

pub fn encrypt_payload(decompressed: &[u8], max_crypt_size: u32) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(decompressed)
        .map_err(|e| ItlError::Compression(e.to_string()))?;
    let mut compressed = encoder
        .finish()
        .map_err(|e| ItlError::Compression(e.to_string()))?;

    let payload_len = compressed.len();
    let aligned = payload_len & !0xf;
    let crypt_size = aligned.min(max_crypt_size as usize);

    let cipher = Aes128::new(GenericArray::from_slice(AES_KEY));
    for chunk in compressed[..crypt_size].chunks_exact_mut(BLOCK_SIZE) {
        cipher.encrypt_block(GenericArray::from_mut_slice(chunk));
    }

    Ok(compressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload_200() -> Vec<u8> {
        b"Hello, iTunes Library!"
            .iter()
            .cycle()
            .take(200)
            .copied()
            .collect()
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let original = sample_payload_200();
        let encrypted = encrypt_payload(&original, 1024).unwrap();
        let decrypted = decrypt_payload(&encrypted, 1024).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn encrypt_decrypt_large_max_crypt() {
        let original = sample_payload_200();
        let encrypted = encrypt_payload(&original, 1_000_000).unwrap();
        let decrypted = decrypt_payload(&encrypted, 1_000_000).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn encrypt_decrypt_small_max_crypt() {
        let original = sample_payload_200();
        let encrypted = encrypt_payload(&original, 16).unwrap();
        let decrypted = decrypt_payload(&encrypted, 16).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn encrypt_decrypt_zero_max_crypt() {
        let original = sample_payload_200();
        let encrypted = encrypt_payload(&original, 0).unwrap();
        let decrypted = decrypt_payload(&encrypted, 0).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn empty_payload() {
        let original: &[u8] = b"";
        let encrypted = encrypt_payload(original, 1024).unwrap();
        let decrypted = decrypt_payload(&encrypted, 1024).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn decrypt_invalid_data() {
        let garbage = [0xdeu8, 0xadu8, 0xd0u8, 0x0du8, 0xfeu8, 0xedu8];
        let result = decrypt_payload(&garbage, 1024);
        assert!(matches!(
            result,
            Err(crate::error::ItlError::Decompression(_))
        ));
    }

    #[test]
    fn deterministic_compression() {
        let data = sample_payload_200();
        let max_crypt = 512u32;
        let a = encrypt_payload(&data, max_crypt).unwrap();
        let b = encrypt_payload(&data, max_crypt).unwrap();
        assert_eq!(a, b);
    }
}
