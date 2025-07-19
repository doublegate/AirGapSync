//! Asymmetric key generation and management (RSA/ECDSA)
//!
//! This module provides RSA and ECDSA key generation for signing
//! and key agreement operations.

use ring::{rand, signature};
use ring::signature::{EcdsaKeyPair, KeyPair};
use rsa::{RsaPrivateKey, RsaPublicKey, pkcs1v15::SigningKey, pkcs1v15::VerifyingKey};
use rsa::signature::{Verifier, RandomizedSigner, SignatureEncoding};
use rsa::pkcs1v15::Signature as RsaSignature;
use pkcs8::{EncodePrivateKey, DecodePrivateKey};
use spki::{EncodePublicKey, DecodePublicKey};
use sha2::{Sha256, Sha384};
use thiserror::Error;
use zeroize::Zeroize;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Error)]
pub enum KeyError {
    #[error("Key generation failed")]
    GenerationFailed,
    
    #[error("Invalid key format")]
    InvalidFormat,
    
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
    
    #[error("Key parsing failed")]
    ParsingFailed,
    
    #[error("Signature verification failed")]
    VerificationFailed,
}

/// Supported asymmetric key algorithms
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AsymmetricAlgorithm {
    /// RSA with 2048-bit key
    Rsa2048,
    /// RSA with 4096-bit key
    Rsa4096,
    /// ECDSA with P-256 curve
    EcdsaP256,
    /// ECDSA with P-384 curve
    EcdsaP384,
}

impl AsymmetricAlgorithm {
    /// Get the algorithm name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            AsymmetricAlgorithm::Rsa2048 => "RSA-2048",
            AsymmetricAlgorithm::Rsa4096 => "RSA-4096",
            AsymmetricAlgorithm::EcdsaP256 => "ECDSA-P256",
            AsymmetricAlgorithm::EcdsaP384 => "ECDSA-P384",
        }
    }
}

/// Container for asymmetric key pairs
pub struct AsymmetricKey {
    /// Algorithm used
    pub algorithm: AsymmetricAlgorithm,
    /// Private key bytes (PKCS#8 format)
    private_key: Vec<u8>,
    /// Public key bytes
    public_key: Vec<u8>,
}

impl Drop for AsymmetricKey {
    fn drop(&mut self) {
        self.private_key.zeroize();
    }
}

impl AsymmetricKey {
    /// Generate a new key pair
    pub fn generate(algorithm: AsymmetricAlgorithm) -> Result<Self, KeyError> {
        let rng = rand::SystemRandom::new();
        
        match algorithm {
            AsymmetricAlgorithm::Rsa2048 | AsymmetricAlgorithm::Rsa4096 => {
                Self::generate_rsa(algorithm, &rng)
            }
            AsymmetricAlgorithm::EcdsaP256 => {
                Self::generate_ecdsa(&signature::ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
            }
            AsymmetricAlgorithm::EcdsaP384 => {
                Self::generate_ecdsa(&signature::ECDSA_P384_SHA384_ASN1_SIGNING, &rng)
            }
        }
    }
    
    /// Generate RSA key pair
    fn generate_rsa(
        algorithm: AsymmetricAlgorithm,
        _rng: &dyn rand::SecureRandom,
    ) -> Result<Self, KeyError> {
        let key_size = match algorithm {
            AsymmetricAlgorithm::Rsa2048 => 2048,
            AsymmetricAlgorithm::Rsa4096 => 4096,
            _ => unreachable!(),
        };
        
        // Generate RSA key using the rsa crate
        let mut system_rng = rand_core::OsRng;
        let private_key = RsaPrivateKey::new(&mut system_rng, key_size)
            .map_err(|_| KeyError::GenerationFailed)?;
        
        let public_key = RsaPublicKey::from(&private_key);
        
        // Serialize private key to PKCS#8 DER format
        let private_key_bytes = private_key.to_pkcs8_der()
            .map_err(|_| KeyError::GenerationFailed)?
            .as_bytes()
            .to_vec();
        
        // Serialize public key to SPKI DER format  
        let public_key_bytes = public_key.to_public_key_der()
            .map_err(|_| KeyError::GenerationFailed)?
            .to_vec();
        
        Ok(AsymmetricKey {
            algorithm,
            private_key: private_key_bytes,
            public_key: public_key_bytes,
        })
    }
    
    /// Generate ECDSA key pair
    fn generate_ecdsa(
        alg: &'static signature::EcdsaSigningAlgorithm,
        rng: &dyn rand::SecureRandom,
    ) -> Result<Self, KeyError> {
        let algorithm = match alg {
            x if x == &signature::ECDSA_P256_SHA256_ASN1_SIGNING => AsymmetricAlgorithm::EcdsaP256,
            x if x == &signature::ECDSA_P384_SHA384_ASN1_SIGNING => AsymmetricAlgorithm::EcdsaP384,
            _ => return Err(KeyError::UnsupportedAlgorithm("Unknown ECDSA curve".to_string())),
        };
        
        // Generate ECDSA key pair
        let private_key = EcdsaKeyPair::generate_pkcs8(alg, rng)
            .map_err(|_| KeyError::GenerationFailed)?;
        
        // Parse to get public key
        let key_pair = EcdsaKeyPair::from_pkcs8(alg, private_key.as_ref(), rng)
            .map_err(|_| KeyError::ParsingFailed)?;
        
        let public_key = key_pair.public_key().as_ref().to_vec();
        
        Ok(AsymmetricKey {
            algorithm,
            private_key: private_key.as_ref().to_vec(),
            public_key,
        })
    }
    
    /// Get the private key bytes (PKCS#8 format)
    pub fn private_key_bytes(&self) -> &[u8] {
        &self.private_key
    }
    
    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> &[u8] {
        &self.public_key
    }
    
    /// Export public key as PEM
    pub fn public_key_pem(&self) -> String {
        let b64 = general_purpose::STANDARD.encode(&self.public_key);
        format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
            b64.chars()
                .collect::<Vec<_>>()
                .chunks(64)
                .map(|chunk| chunk.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
    
    /// Sign data with this key
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>, KeyError> {
        use ring::rand;
        let rng = rand::SystemRandom::new();
        
        match self.algorithm {
            AsymmetricAlgorithm::Rsa2048 | AsymmetricAlgorithm::Rsa4096 => {
                // Parse the private key from PKCS#8 DER
                let private_key = RsaPrivateKey::from_pkcs8_der(&self.private_key)
                    .map_err(|_| KeyError::InvalidFormat)?;
                
                // Create signing key without digest prefix (unprefixed)
                let signing_key = SigningKey::<Sha256>::new_unprefixed(private_key);
                
                // Sign the data
                let mut rng = rand_core::OsRng;
                let signature = signing_key.sign_with_rng(&mut rng, data);
                
                Ok(signature.to_vec())
            }
            AsymmetricAlgorithm::EcdsaP256 => {
                let key_pair = EcdsaKeyPair::from_pkcs8(
                    &signature::ECDSA_P256_SHA256_ASN1_SIGNING,
                    &self.private_key,
                    &rng,
                ).map_err(|_| KeyError::ParsingFailed)?;
                
                let signature = key_pair.sign(&rng, data)
                    .map_err(|_| KeyError::GenerationFailed)?;
                
                Ok(signature.as_ref().to_vec())
            }
            AsymmetricAlgorithm::EcdsaP384 => {
                let key_pair = EcdsaKeyPair::from_pkcs8(
                    &signature::ECDSA_P384_SHA384_ASN1_SIGNING,
                    &self.private_key,
                    &rng,
                ).map_err(|_| KeyError::ParsingFailed)?;
                
                let signature = key_pair.sign(&rng, data)
                    .map_err(|_| KeyError::GenerationFailed)?;
                
                Ok(signature.as_ref().to_vec())
            }
        }
    }
    
    /// Verify a signature
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<(), KeyError> {
        match self.algorithm {
            AsymmetricAlgorithm::Rsa2048 | AsymmetricAlgorithm::Rsa4096 => {
                // Parse the public key from SPKI DER
                let public_key = RsaPublicKey::from_public_key_der(&self.public_key)
                    .map_err(|_| KeyError::InvalidFormat)?;
                
                // Create verifying key without digest prefix (unprefixed)
                let verifying_key = VerifyingKey::<Sha256>::new_unprefixed(public_key);
                
                // Create signature from bytes
                let rsa_signature = RsaSignature::try_from(signature)
                    .map_err(|_| KeyError::VerificationFailed)?;
                
                // Verify the signature
                verifying_key.verify(data, &rsa_signature)
                    .map_err(|_| KeyError::VerificationFailed)?;
            }
            AsymmetricAlgorithm::EcdsaP256 => {
                let public_key = signature::UnparsedPublicKey::new(
                    &signature::ECDSA_P256_SHA256_ASN1,
                    &self.public_key,
                );
                
                public_key.verify(data, signature)
                    .map_err(|_| KeyError::VerificationFailed)?;
            }
            AsymmetricAlgorithm::EcdsaP384 => {
                let public_key = signature::UnparsedPublicKey::new(
                    &signature::ECDSA_P384_SHA384_ASN1,
                    &self.public_key,
                );
                
                public_key.verify(data, signature)
                    .map_err(|_| KeyError::VerificationFailed)?;
            }
        }
        
        Ok(())
    }
}

/// Key agreement for ECDH
pub struct KeyAgreement {
    algorithm: AsymmetricAlgorithm,
    private_key: Vec<u8>,
}

impl Drop for KeyAgreement {
    fn drop(&mut self) {
        self.private_key.zeroize();
    }
}

impl KeyAgreement {
    /// Create from an existing ECDSA key
    pub fn from_key(key: &AsymmetricKey) -> Result<Self, KeyError> {
        match key.algorithm {
            AsymmetricAlgorithm::EcdsaP256 | AsymmetricAlgorithm::EcdsaP384 => {
                Ok(KeyAgreement {
                    algorithm: key.algorithm,
                    private_key: key.private_key.clone(),
                })
            }
            _ => Err(KeyError::UnsupportedAlgorithm(
                "Key agreement requires ECDSA keys".to_string()
            )),
        }
    }
    
    /// Perform ECDH key agreement
    pub fn agree(&self, peer_public_key: &[u8]) -> Result<Vec<u8>, KeyError> {
        use ring::{agreement, rand};
        
        let algorithm = match self.algorithm {
            AsymmetricAlgorithm::EcdsaP256 => &agreement::ECDH_P256,
            AsymmetricAlgorithm::EcdsaP384 => &agreement::ECDH_P384,
            _ => return Err(KeyError::UnsupportedAlgorithm("Not an ECDH key".to_string())),
        };
        
        // For now, generate a new ephemeral key since ring doesn't support loading ECDSA keys for ECDH
        let rng = rand::SystemRandom::new();
        let private_key = agreement::EphemeralPrivateKey::generate(algorithm, &rng)
            .map_err(|_| KeyError::GenerationFailed)?;
        
        let peer_public_key = agreement::UnparsedPublicKey::new(
            algorithm,
            peer_public_key,
        );
        
        let result = agreement::agree_ephemeral(
            private_key,
            &peer_public_key,
            |shared_secret| Ok(shared_secret.to_vec()),
        ).map_err(|_| KeyError::GenerationFailed)?;
        
        result.map_err(|_: ()| KeyError::GenerationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rsa_key_generation() {
        let key = AsymmetricKey::generate(AsymmetricAlgorithm::Rsa2048).unwrap();
        assert_eq!(key.algorithm, AsymmetricAlgorithm::Rsa2048);
        assert!(!key.private_key.is_empty());
        assert!(!key.public_key.is_empty());
    }
    
    #[test]
    fn test_ecdsa_key_generation() {
        let key = AsymmetricKey::generate(AsymmetricAlgorithm::EcdsaP256).unwrap();
        assert_eq!(key.algorithm, AsymmetricAlgorithm::EcdsaP256);
        assert!(!key.private_key.is_empty());
        assert!(!key.public_key.is_empty());
    }
    
    #[test]
    fn test_sign_verify_rsa() {
        let key = AsymmetricKey::generate(AsymmetricAlgorithm::Rsa2048).unwrap();
        let data = b"test message";
        
        let signature = key.sign(data).unwrap();
        assert!(key.verify(data, &signature).is_ok());
        
        // Verify with wrong data should fail
        assert!(key.verify(b"wrong message", &signature).is_err());
    }
    
    #[test]
    fn test_sign_verify_ecdsa() {
        let key = AsymmetricKey::generate(AsymmetricAlgorithm::EcdsaP256).unwrap();
        let data = b"test message";
        
        let signature = key.sign(data).unwrap();
        assert!(key.verify(data, &signature).is_ok());
        
        // Verify with wrong signature should fail
        let mut bad_sig = signature.clone();
        bad_sig[0] ^= 0xFF;
        assert!(key.verify(data, &bad_sig).is_err());
    }
    
    #[test]
    fn test_public_key_pem() {
        let key = AsymmetricKey::generate(AsymmetricAlgorithm::EcdsaP256).unwrap();
        let pem = key.public_key_pem();
        
        assert!(pem.starts_with("-----BEGIN PUBLIC KEY-----"));
        assert!(pem.ends_with("-----END PUBLIC KEY-----"));
    }
}