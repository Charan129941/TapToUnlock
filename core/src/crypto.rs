use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Key, Nonce as AesNonce,
};
use chrono::Utc;
use ed25519_dalek::{
    Signature, Signer, SigningKey, Verifier, VerifyingKey,
    SECRET_KEY_LENGTH, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH,
};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use x25519_dalek::{PublicKey as EcdhPublicKey, StaticSecret};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum CryptoError {
    #[error("Invalid key length or format")]
    InvalidKeyFormat,
    #[error("Signature verification failed: unauthorized or tampered payload")]
    SignatureVerificationFailed,
    #[error("Encryption failure: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failure: authentication tag mismatch or malformed ciphertext")]
    DecryptionFailed,
    #[error("Replay attack detected: nonce has already been consumed")]
    ReplayAttackDetected,
    #[error("Timestamp drift exceeded: payload timestamp is outside acceptable window ({0} ms drift)")]
    TimestampDriftExceeded(i64),
    #[error("Base64 decoding error")]
    Base64Error,
}

/// Manages Ed25519 asymmetric signing and verification keys.
pub struct KeyPairManager {
    signing_key: SigningKey,
}

impl KeyPairManager {
    /// Generates a new cryptographically secure random Ed25519 keypair.
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Self { signing_key }
    }

    /// Reconstructs a KeyPairManager from raw 32-byte private key bytes.
    pub fn from_bytes(secret_bytes: &[u8]) -> Result<Self, CryptoError> {
        if secret_bytes.len() != SECRET_KEY_LENGTH {
            return Err(CryptoError::InvalidKeyFormat);
        }
        let mut key_bytes = [0u8; SECRET_KEY_LENGTH];
        key_bytes.copy_from_slice(secret_bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        Ok(Self { signing_key })
    }

    /// Returns the raw 32-byte private key.
    pub fn secret_bytes(&self) -> [u8; SECRET_KEY_LENGTH] {
        self.signing_key.to_bytes()
    }

    /// Returns the associated 32-byte public verifying key.
    pub fn public_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn from_secret_bytes(secret_bytes: &[u8]) -> Result<Self, CryptoError> {
        Self::from_bytes(secret_bytes)
    }

    pub fn secret_key_bytes(&self) -> [u8; SECRET_KEY_LENGTH] {
        self.secret_bytes()
    }

    /// Signs a message slice, returning a 64-byte Ed25519 signature.
    pub fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_LENGTH] {
        let signature: Signature = self.signing_key.sign(message);
        signature.to_bytes()
    }

    /// Verifies a message against this instance's public key (returns bool for tests/validation).
    pub fn verify(&self, message: &[u8], signature_bytes: &[u8]) -> bool {
        Self::verify_slice(&self.public_key().to_bytes(), message, signature_bytes).is_ok()
    }

    /// Verifies a message against a public key slice and a 64-byte signature.
    pub fn verify_slice(
        public_key_bytes: &[u8],
        message: &[u8],
        signature_bytes: &[u8],
    ) -> Result<(), CryptoError> {
        if public_key_bytes.len() != PUBLIC_KEY_LENGTH || signature_bytes.len() != SIGNATURE_LENGTH {
            return Err(CryptoError::InvalidKeyFormat);
        }
        let mut pk_buf = [0u8; PUBLIC_KEY_LENGTH];
        pk_buf.copy_from_slice(public_key_bytes);
        let verifying_key = VerifyingKey::from_bytes(&pk_buf)
            .map_err(|_| CryptoError::InvalidKeyFormat)?;

        let mut sig_buf = [0u8; SIGNATURE_LENGTH];
        sig_buf.copy_from_slice(signature_bytes);
        let signature = Signature::from_bytes(&sig_buf);

        verifying_key
            .verify(message, &signature)
            .map_err(|_| CryptoError::SignatureVerificationFailed)
    }
}

/// Manages X25519 Diffie-Hellman ephemeral key exchange for secure transport encryption.
pub struct EcdhManager {
    secret: StaticSecret,
}

impl EcdhManager {
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        Self { secret }
    }

    pub fn public_key(&self) -> EcdhPublicKey {
        EcdhPublicKey::from(&self.secret)
    }

    /// Derives a 32-byte AES-GCM encryption key from peer's public key using SHA-256 HKDF/hash.
    pub fn derive_shared_key(&self, peer_pub_bytes: &[u8]) -> Result<[u8; 32], CryptoError> {
        if peer_pub_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyFormat);
        }
        let mut buf = [0u8; 32];
        buf.copy_from_slice(peer_pub_bytes);
        let peer_pub = EcdhPublicKey::from(buf);

        let shared_secret = self.secret.diffie_hellman(&peer_pub);
        
        // Hash the Diffie-Hellman secret to derive a uniform 256-bit key
        let mut hasher = Sha256::new();
        hasher.update(shared_secret.as_bytes());
        hasher.update(b"opentap_ecdh_key_derivation_v1");
        let result = hasher.finalize();
        
        let mut derived_key = [0u8; 32];
        derived_key.copy_from_slice(&result[..]);
        Ok(derived_key)
    }
}

/// Provides AES-256-GCM authenticated encryption and decryption.
pub struct SymmetricCipher;

impl SymmetricCipher {
    /// Encrypts plaintext with additional authenticated data (AAD) using a 32-byte key.
    /// Returns: (12-byte nonce || ciphertext || 16-byte auth tag).
    pub fn encrypt(key_bytes: &[u8], plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if key_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyFormat);
        }
        let key = Key::<Aes256Gcm>::from_slice(key_bytes);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = AesNonce::from_slice(&nonce_bytes);

        let payload = Payload {
            msg: plaintext,
            aad,
        };

        let ciphertext = cipher
            .encrypt(nonce, payload)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypts a payload produced by `encrypt`.
    pub fn decrypt(key_bytes: &[u8], encrypted_data: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if key_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyFormat);
        }
        if encrypted_data.len() < 12 + 16 {
            return Err(CryptoError::DecryptionFailed);
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let key = Key::<Aes256Gcm>::from_slice(key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = AesNonce::from_slice(nonce_bytes);

        let payload = Payload {
            msg: ciphertext,
            aad,
        };

        cipher
            .decrypt(nonce, payload)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

/// Thread-safe Nonce and Timestamp Validator to prevent replay attacks and timestamp manipulation.
#[derive(Clone)]
pub struct NonceValidator {
    /// Maps Nonce bytes -> Timestamp (UTC millis) when it was received.
    seen_nonces: Arc<Mutex<HashMap<Vec<u8>, i64>>>,
    /// Maximum acceptable clock drift in milliseconds (default: 5000ms = 5s).
    max_drift_ms: i64,
    /// Time-to-live for nonces in cache in milliseconds (default: 60000ms = 60s).
    ttl_ms: i64,
}

impl NonceValidator {
    pub fn new(max_drift_ms: i64, ttl_ms: i64) -> Self {
        Self {
            seen_nonces: Arc::new(Mutex::new(HashMap::new())),
            max_drift_ms,
            ttl_ms,
        }
    }

    /// Generates a cryptographically random 16-byte nonce.
    pub fn generate_nonce() -> [u8; 16] {
        let mut buf = [0u8; 16];
        OsRng.fill_bytes(&mut buf);
        buf
    }

    /// Validates an incoming request timestamp and nonce against replay attacks.
    /// Returns Ok(()) if the request is fresh and unique.
    pub fn validate(&self, nonce: &[u8], timestamp_millis: i64) -> Result<(), CryptoError> {
        let now_ms = Utc::now().timestamp_millis();
        let drift = (now_ms - timestamp_millis).abs();

        if drift > self.max_drift_ms {
            return Err(CryptoError::TimestampDriftExceeded(drift));
        }

        let mut cache = self.seen_nonces.lock().unwrap();

        // Prune expired nonces
        cache.retain(|_, &mut seen_time| (now_ms - seen_time) <= self.ttl_ms);

        // Check if nonce was already consumed
        if cache.contains_key(nonce) {
            return Err(CryptoError::ReplayAttackDetected);
        }

        // Record fresh nonce
        cache.insert(nonce.to_vec(), now_ms);
        Ok(())
    }

    /// Helper for test matrices and simulation runners to validate nonce uniqueness without absolute clock drift check.
    pub fn validate_and_store(&self, nonce: &[u8], _timestamp_millis: i64) -> bool {
        let mut cache = self.seen_nonces.lock().unwrap();
        if cache.contains_key(nonce) {
            return false;
        }
        cache.insert(nonce.to_vec(), Utc::now().timestamp_millis());
        true
    }

    /// Initializes a NonceValidator from a standard Duration.
    pub fn from_duration(duration: std::time::Duration) -> Self {
        Self::new(duration.as_millis() as i64, duration.as_millis() as i64)
    }
}

impl Default for NonceValidator {
    fn default() -> Self {
        Self::new(5000, 60000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_ed25519_signing_and_verification() {
        let manager = KeyPairManager::generate();
        let message = b"UNLOCK_TARGET_PC_001_TIMESTAMP_1721920000";
        let sig = manager.sign(message);

        let pk_bytes = manager.public_key().to_bytes();
        assert!(KeyPairManager::verify(&pk_bytes, message, &sig).is_ok());

        // Tamper with message
        let tampered_msg = b"UNLOCK_TARGET_PC_002_TIMESTAMP_1721920000";
        assert_eq!(
            KeyPairManager::verify(&pk_bytes, tampered_msg, &sig),
            Err(CryptoError::SignatureVerificationFailed)
        );
    }

    #[test]
    fn test_ecdh_shared_secret_derivation() {
        let alice = EcdhManager::generate();
        let bob = EcdhManager::generate();

        let alice_shared = alice.derive_shared_key(bob.public_key().as_bytes()).unwrap();
        let bob_shared = bob.derive_shared_key(alice.public_key().as_bytes()).unwrap();

        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_aes_gcm_authenticated_encryption() {
        let key = [42u8; 32];
        let plaintext = b"Sensitve biometric unlock payload v1";
        let aad = b"header_data";

        let ciphertext = SymmetricCipher::encrypt(&key, plaintext, aad).unwrap();
        assert_ne!(ciphertext.as_slice(), plaintext);

        let decrypted = SymmetricCipher::decrypt(&key, &ciphertext, aad).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext);

        // Tampering with AAD must fail decryption
        assert_eq!(
            SymmetricCipher::decrypt(&key, &ciphertext, b"tampered_aad"),
            Err(CryptoError::DecryptionFailed)
        );
    }

    #[test]
    fn test_nonce_validator_replay_protection() {
        let validator = NonceValidator::new(5000, 60000);
        let nonce = NonceValidator::generate_nonce();
        let now = Utc::now().timestamp_millis();

        // First attempt should succeed
        assert!(validator.validate(&nonce, now).is_ok());

        // Replay of exact same nonce within valid window must fail
        assert_eq!(
            validator.validate(&nonce, now),
            Err(CryptoError::ReplayAttackDetected)
        );
    }

    #[test]
    fn test_timestamp_drift_rejection() {
        let validator = NonceValidator::new(2000, 60000); // 2 sec max drift
        let nonce = NonceValidator::generate_nonce();
        let stale_time = Utc::now().timestamp_millis() - 5000; // 5 seconds in the past

        match validator.validate(&nonce, stale_time) {
            Err(CryptoError::TimestampDriftExceeded(drift)) => {
                assert!(drift >= 4900);
            }
            other => panic!("Expected TimestampDriftExceeded, got {:?}", other),
        }
    }
}
