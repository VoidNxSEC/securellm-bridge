// Cryptographic primitives module
// Implements: AES-256-GCM encryption/decryption, Argon2 password hashing,
// secure key generation, and key wrapping (password-derived encryption).

use crate::{Result, SecurityError};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, CHACHA20_POLY1305, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};

/// Encryption service for data at rest and in transit
pub struct EncryptionService {
    rng: SystemRandom,
}

/// Strategy for key derivation from passwords
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivation {
    /// Derive a 256-bit key via Argon2id with the given salt
    Argon2id { salt: &'static str },
    /// Use raw key bytes directly (no derivation)
    Raw,
}

impl EncryptionService {
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }

    // ── Key Generation ──────────────────────────────────────────────

    /// Generate a secure random 256-bit key for AES-256-GCM
    pub fn generate_key(&self) -> Result<Vec<u8>> {
        self.random_bytes(32)
    }

    /// Generate a secure random nonce (96 bits for AES-GCM)
    pub fn generate_nonce(&self) -> Result<Vec<u8>> {
        self.random_bytes(NONCE_LEN)
    }

    fn random_bytes(&self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.rng.fill(&mut buf).map_err(|e| {
            SecurityError::Crypto(format!("Failed to generate random bytes: {:?}", e))
        })?;
        Ok(buf)
    }

    // ── AES-256-GCM Encryption ──────────────────────────────────────

    /// Encrypt plaintext using AES-256-GCM.
    ///
    /// # Format (on-wire layout)
    /// `[nonce: 12 bytes][ciphertext + tag: N bytes]`
    ///
    /// The nonce is randomly generated per-encryption and prepended.
    /// AES-GCM appends a 16-byte authentication tag automatically.
    pub fn encrypt(&self, key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        let unbound_key = UnboundKey::new(&AES_256_GCM, key)
            .map_err(|e| SecurityError::Crypto(format!("Invalid key: {:?}", e)))?;

        let nonce = self.generate_nonce()?;
        let nonce_array = Nonce::try_assume_unique_for_key(&nonce)
            .map_err(|e| SecurityError::Crypto(format!("Nonce error: {:?}", e)))?;

        // LessSafeKey is ~2x faster; safe because we generate a unique nonce per encryption
        let sealing_key = LessSafeKey::new(unbound_key);

        let mut in_out = plaintext.to_vec();
        sealing_key
            .seal_in_place_append_tag(nonce_array, Aad::empty(), &mut in_out)
            .map_err(|e| SecurityError::Crypto(format!("Encryption failed: {:?}", e)))?;

        // Prepend nonce for decryption
        let mut output = nonce;
        output.extend_from_slice(&in_out);
        Ok(output)
    }

    /// Decrypt ciphertext produced by [`encrypt`].
    ///
    /// Expects the format `[nonce: 12 bytes][ciphertext + tag: N bytes]`.
    /// Returns `Err` if authentication fails (tampered data, wrong key).
    pub fn decrypt(&self, key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < NONCE_LEN + 16 {
            return Err(SecurityError::Crypto(
                "Ciphertext too short: must contain at least nonce (12 bytes) + tag (16 bytes)"
                    .to_string(),
            ));
        }

        let unbound_key = UnboundKey::new(&AES_256_GCM, key)
            .map_err(|e| SecurityError::Crypto(format!("Invalid key: {:?}", e)))?;

        let (nonce_bytes, encrypted) = ciphertext.split_at(NONCE_LEN);
        let nonce_array = Nonce::try_assume_unique_for_key(nonce_bytes)
            .map_err(|e| SecurityError::Crypto(format!("Nonce error: {:?}", e)))?;

        let opening_key = LessSafeKey::new(unbound_key);

        let mut in_out = encrypted.to_vec();
        let plaintext = opening_key
            .open_in_place(nonce_array, Aad::empty(), &mut in_out)
            .map_err(|e| {
                SecurityError::Crypto(format!(
                    "Decryption failed (wrong key or tampered data): {:?}",
                    e
                ))
            })?;

        Ok(plaintext.to_vec())
    }

    /// Convenience: encrypt with a randomly generated key (returned alongside ciphertext)
    pub fn encrypt_with_random_key(&self, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let key = self.generate_key()?;
        let ciphertext = self.encrypt(&key, plaintext)?;
        Ok((key, ciphertext))
    }

    // ── ChaCha20-Poly1305 (alternative for environments without AES-NI) ──

    /// Encrypt using ChaCha20-Poly1305 (faster on CPUs without AES-NI)
    pub fn encrypt_chacha(&self, key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        let unbound_key = UnboundKey::new(&CHACHA20_POLY1305, key)
            .map_err(|e| SecurityError::Crypto(format!("Invalid key: {:?}", e)))?;

        let nonce = self.random_bytes(NONCE_LEN)?;
        let nonce_array = Nonce::try_assume_unique_for_key(&nonce)
            .map_err(|e| SecurityError::Crypto(format!("Nonce error: {:?}", e)))?;

        let sealing_key = LessSafeKey::new(unbound_key);

        let mut in_out = plaintext.to_vec();
        sealing_key
            .seal_in_place_append_tag(nonce_array, Aad::empty(), &mut in_out)
            .map_err(|e| SecurityError::Crypto(format!("ChaCha encryption failed: {:?}", e)))?;

        let mut output = nonce;
        output.extend_from_slice(&in_out);
        Ok(output)
    }

    /// Decrypt ChaCha20-Poly1305 ciphertext
    pub fn decrypt_chacha(&self, key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < NONCE_LEN + 16 {
            return Err(SecurityError::Crypto(
                "Ciphertext too short for ChaCha20-Poly1305".to_string(),
            ));
        }

        let unbound_key = UnboundKey::new(&CHACHA20_POLY1305, key)
            .map_err(|e| SecurityError::Crypto(format!("Invalid key: {:?}", e)))?;

        let (nonce_bytes, encrypted) = ciphertext.split_at(NONCE_LEN);
        let nonce_array = Nonce::try_assume_unique_for_key(nonce_bytes)
            .map_err(|e| SecurityError::Crypto(format!("Nonce error: {:?}", e)))?;

        let opening_key = LessSafeKey::new(unbound_key);

        let mut in_out = encrypted.to_vec();
        let plaintext = opening_key
            .open_in_place(nonce_array, Aad::empty(), &mut in_out)
            .map_err(|e| SecurityError::Crypto(format!("ChaCha decryption failed: {:?}", e)))?;

        Ok(plaintext.to_vec())
    }

    // ── Key Wrapping (Password-Derived Encryption) ──────────────────

    /// Derive a 256-bit key from a password using Argon2id.
    ///
    /// Uses a fixed salt — suitable for **key wrapping**, NOT for password storage.
    /// For password storage, use the standalone `hash_password` / `verify_password`.
    pub fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<Vec<u8>> {
        use argon2::{password_hash::SaltString, Argon2};

        let salt_string = SaltString::encode_b64(salt)
            .map_err(|e| SecurityError::Crypto(format!("Salt encoding failed: {}", e)))?;

        let argon2 = Argon2::default();
        let mut key = vec![0u8; 32];

        argon2
            .hash_password_into(
                password.as_bytes(),
                salt_string.as_str().as_bytes(),
                &mut key,
            )
            .map_err(|e| SecurityError::Crypto(format!("Key derivation failed: {}", e)))?;

        Ok(key)
    }

    /// Encrypt plaintext with a password (derives key via Argon2id + random salt).
    ///
    /// Format: `[salt: 16 bytes][nonce: 12 bytes][ciphertext + tag]`
    pub fn encrypt_with_password(&self, password: &str, plaintext: &[u8]) -> Result<Vec<u8>> {
        let salt = self.random_bytes(16)?;
        let key = Self::derive_key_from_password(password, &salt)?;

        let encrypted = self.encrypt(&key, plaintext)?;

        // encrypted already has [nonce][ciphertext+tag], prepend salt
        let mut output = salt;
        output.extend_from_slice(&encrypted);
        Ok(output)
    }

    /// Decrypt ciphertext produced by [`encrypt_with_password`].
    pub fn decrypt_with_password(&self, password: &str, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 16 + NONCE_LEN + 16 {
            return Err(SecurityError::Crypto(
                "Password-encrypted data too short".to_string(),
            ));
        }

        let (salt, rest) = ciphertext.split_at(16);
        let key = Self::derive_key_from_password(password, salt)?;
        self.decrypt(&key, rest)
    }
}

impl Default for EncryptionService {
    fn default() -> Self {
        Self::new()
    }
}

// ── Standalone Password Hashing (Argon2id) ──────────────────────────

/// Hash passwords securely using Argon2id.
///
/// Uses a random salt per hash. Suitable for password storage / verification.
/// Does NOT share salts with the key-wrapping functions.
pub fn hash_password(password: &str) -> Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| SecurityError::Crypto(format!("Failed to hash password: {}", e)))?;

    Ok(hash.to_string())
}

/// Verify password against an Argon2id hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| SecurityError::Crypto(format!("Invalid hash format: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Key Generation ──────────────────────────────────────────

    #[test]
    fn test_key_generation() {
        let service = EncryptionService::new();
        let key1 = service.generate_key().unwrap();
        let key2 = service.generate_key().unwrap();

        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
        assert_ne!(key1, key2, "Keys should be unique");
    }

    #[test]
    fn test_nonce_generation() {
        let service = EncryptionService::new();
        let nonce1 = service.generate_nonce().unwrap();
        let nonce2 = service.generate_nonce().unwrap();

        assert_eq!(nonce1.len(), NONCE_LEN);
        assert_eq!(nonce2.len(), NONCE_LEN);
        assert_ne!(nonce1, nonce2, "Nonces should be unique");
    }

    // ── AES-256-GCM Encrypt/Decrypt ────────────────────────────

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let service = EncryptionService::new();
        let key = service.generate_key().unwrap();
        let plaintext = b"The quick brown fox jumps over the lazy dog";

        let ciphertext = service.encrypt(&key, plaintext).unwrap();
        // Should be nonce(12) + plaintext + tag(16)
        assert_eq!(ciphertext.len(), NONCE_LEN + plaintext.len() + 16);
        assert_ne!(ciphertext, plaintext);

        let decrypted = service.decrypt(&key, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_empty_plaintext() {
        let service = EncryptionService::new();
        let key = service.generate_key().unwrap();

        let ciphertext = service.encrypt(&key, b"").unwrap();
        // nonce(12) + empty + tag(16) = 28
        assert_eq!(ciphertext.len(), NONCE_LEN + 16);

        let decrypted = service.decrypt(&key, &ciphertext).unwrap();
        assert_eq!(decrypted, b"");
    }

    #[test]
    fn test_encrypt_large_plaintext() {
        let service = EncryptionService::new();
        let key = service.generate_key().unwrap();
        let plaintext = vec![0x42u8; 1_000_000]; // 1 MB

        let ciphertext = service.encrypt(&key, &plaintext).unwrap();
        assert_eq!(ciphertext.len(), NONCE_LEN + plaintext.len() + 16);

        let decrypted = service.decrypt(&key, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let service = EncryptionService::new();
        let key1 = service.generate_key().unwrap();
        let key2 = service.generate_key().unwrap();
        let plaintext = b"secret message";

        let ciphertext = service.encrypt(&key1, plaintext).unwrap();
        let result = service.decrypt(&key2, &ciphertext);

        assert!(result.is_err(), "Should fail with wrong key");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Decryption failed") || err.contains("tampered"));
    }

    #[test]
    fn test_decrypt_tampered_data() {
        let service = EncryptionService::new();
        let key = service.generate_key().unwrap();
        let plaintext = b"tamper me";

        let mut ciphertext = service.encrypt(&key, plaintext).unwrap();
        // Flip a bit in the ciphertext (after nonce)
        ciphertext[NONCE_LEN + 3] ^= 0x01;

        let result = service.decrypt(&key, &ciphertext);
        assert!(result.is_err(), "Should detect tampering");
    }

    #[test]
    fn test_decrypt_truncated() {
        let service = EncryptionService::new();
        let key = service.generate_key().unwrap();
        let plaintext = b"truncate me";

        let ciphertext = service.encrypt(&key, plaintext).unwrap();
        let truncated = &ciphertext[..NONCE_LEN]; // Only nonce, no ciphertext

        let result = service.decrypt(&key, truncated);
        assert!(result.is_err(), "Should reject truncated data");
    }

    #[test]
    fn test_encrypt_different_nonces() {
        let service = EncryptionService::new();
        let key = service.generate_key().unwrap();
        let plaintext = b"same data";

        let ct1 = service.encrypt(&key, plaintext).unwrap();
        let ct2 = service.encrypt(&key, plaintext).unwrap();

        // Same plaintext + same key should produce different ciphertexts
        // because nonces are random
        assert_ne!(ct1, ct2, "Ciphertexts should differ due to unique nonces");
    }

    #[test]
    fn test_encrypt_with_random_key() {
        let service = EncryptionService::new();
        let plaintext = b"ephemeral key";

        let (key, ciphertext) = service.encrypt_with_random_key(plaintext).unwrap();
        assert_eq!(key.len(), 32);

        let decrypted = service.decrypt(&key, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    // ── ChaCha20-Poly1305 ───────────────────────────────────────

    #[test]
    fn test_chacha_roundtrip() {
        let service = EncryptionService::new();
        let key = service.generate_key().unwrap();
        let plaintext = b"ChaCha20-Poly1305 is great for mobile devices";

        let ciphertext = service.encrypt_chacha(&key, plaintext).unwrap();
        assert_eq!(ciphertext.len(), NONCE_LEN + plaintext.len() + 16);

        let decrypted = service.decrypt_chacha(&key, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_chacha_wrong_key() {
        let service = EncryptionService::new();
        let key1 = service.generate_key().unwrap();
        let key2 = service.generate_key().unwrap();

        let ct = service.encrypt_chacha(&key1, b"secret").unwrap();
        assert!(service.decrypt_chacha(&key2, &ct).is_err());
    }

    // ── Password-Based Encryption ───────────────────────────────

    #[test]
    fn test_password_encrypt_decrypt_roundtrip() {
        let service = EncryptionService::new();
        let password = "my-super-secret-password";
        let plaintext = b"Password-protected data";

        let ciphertext = service.encrypt_with_password(password, plaintext).unwrap();
        // Format: salt(16) + nonce(12) + plaintext + tag(16)
        assert_eq!(ciphertext.len(), 16 + NONCE_LEN + plaintext.len() + 16);

        let decrypted = service
            .decrypt_with_password(password, &ciphertext)
            .unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_password_decrypt_wrong_password() {
        let service = EncryptionService::new();

        let ct = service
            .encrypt_with_password("correct-password", b"data")
            .unwrap();

        let result = service.decrypt_with_password("wrong-password", &ct);
        assert!(result.is_err(), "Should fail with wrong password");
    }

    #[test]
    fn test_password_encrypt_unique_outputs() {
        let service = EncryptionService::new();
        let password = "same-password";
        let plaintext = b"same data";

        let ct1 = service.encrypt_with_password(password, plaintext).unwrap();
        let ct2 = service.encrypt_with_password(password, plaintext).unwrap();

        // Different salts → different outputs
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn test_derive_key_deterministic() {
        let password = "deterministic-key";
        let salt = b"fixed-salt-12345";

        let key1 = EncryptionService::derive_key_from_password(password, salt).unwrap();
        let key2 = EncryptionService::derive_key_from_password(password, salt).unwrap();

        assert_eq!(key1, key2, "Same password + salt should produce same key");
    }

    // ── Password Hashing (standalone) ───────────────────────────

    #[test]
    fn test_password_hashing() {
        let password = "secure_password_123";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_password_hashing_different_salts() {
        let password = "same-password";

        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        assert_ne!(
            hash1, hash2,
            "Different salts should produce different hashes"
        );
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    // ── Cross-algorithm safety ─────────────────────────────────

    #[test]
    fn test_aes_ciphertext_not_decryptable_as_chacha() {
        let service = EncryptionService::new();
        let key = service.generate_key().unwrap();

        let aes_ct = service.encrypt(&key, b"cross algo").unwrap();
        // AES ciphertext should not decrypt as ChaCha (and vice versa)
        let result = service.decrypt_chacha(&key, &aes_ct);
        assert!(
            result.is_err(),
            "AES ciphertext should not decrypt as ChaCha"
        );
    }

    #[test]
    fn test_invalid_key_length() {
        let service = EncryptionService::new();
        let short_key = vec![0u8; 16]; // AES-256-GCM requires 32 bytes

        let result = service.encrypt(&short_key, b"test");
        assert!(result.is_err(), "Should reject 16-byte key for AES-256-GCM");
    }
}
