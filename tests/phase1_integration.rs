//! Phase 1 Integration Tests
//!
//! Tests for configuration, keychain, and cryptography integration

use std::path::PathBuf;

#[test]
fn test_phase1_full_integration() {
    // Initialize library
    airgap_sync::initialize().expect("Failed to initialize library");

    // Test configuration
    test_configuration_system();

    // Test keychain integration
    #[cfg(target_os = "macos")]
    test_keychain_integration();

    // Test cryptography
    test_crypto_integration();

    // Test key generation
    test_asymmetric_keys();
}

fn test_configuration_system() {
    use airgap_sync::config::*;

    // Create a test configuration
    let config = Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: PathBuf::from("/tmp"),
            exclude: vec!["*.tmp".to_string()],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "TEST001".to_string(),
            name: "Test Device".to_string(),
            mount_point: PathBuf::from("/tmp/test-mount"),
            encryption: EncryptionConfig {
                algorithm: EncryptionAlgorithm::Aes256Gcm,
                key_derivation: KeyDerivation::Pbkdf2,
                iterations: 100_000,
            },
        }],
        policy: PolicyConfig::default(),
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    // Test serialization
    let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize config");
    assert!(toml_str.contains("[source]"));
    assert!(toml_str.contains("[[device]]"));

    // Test deserialization
    let parsed: Config = toml::from_str(&toml_str).expect("Failed to parse config");
    assert_eq!(parsed.device.len(), 1);
    assert_eq!(parsed.device[0].id, "TEST001");

    // Test JSON schema generation
    use airgap_sync::schema::*;
    let schema = generate_config_schema().expect("Failed to generate schema");
    assert!(schema.is_object());

    // Test validation
    let config_json = serde_json::to_value(&config).expect("Failed to convert to JSON");
    validate_config_json(&config_json).expect("Validation should pass");
}

#[cfg(target_os = "macos")]
fn test_keychain_integration() {
    use airgap_sync::keychain::*;

    let keychain = KeychainManager::with_service_name("com.airgapsync.test".to_string());
    let device_id = "TEST-DEVICE-001";

    // Generate a test key
    let key = generate_key("AES-256", device_id).expect("Failed to generate key");
    assert_eq!(key.key_material.len(), 32);
    assert_eq!(key.metadata.algorithm, "AES-256");

    // Store in keychain (may require user authorization)
    if keychain.store_key(device_id, &key).is_ok() {
        // Retrieve key
        let retrieved = keychain.get_key(device_id).expect("Failed to retrieve key");
        assert_eq!(retrieved.key_material, key.key_material);
        assert_eq!(retrieved.metadata.algorithm, key.metadata.algorithm);

        // Test key rotation
        let rotated = rotate_key(&keychain, device_id).expect("Failed to rotate key");
        assert_ne!(rotated.key_material, key.key_material);
        assert_eq!(rotated.metadata.version, key.metadata.version + 1);

        // Clean up
        let _ = keychain.delete_key(device_id);
    }
}

fn test_crypto_integration() {
    use airgap_sync::crypto::*;

    // Test AES-256-GCM
    let key = CryptoKey::generate(Algorithm::Aes256Gcm).expect("Failed to generate key");
    let plaintext = b"This is a test message for AES-256-GCM encryption";
    let aad = b"metadata";

    let ciphertext = encrypt(&key, plaintext, aad).expect("Encryption failed");
    assert!(ciphertext.len() > plaintext.len());

    let decrypted = decrypt(&key, &ciphertext, aad).expect("Decryption failed");
    assert_eq!(decrypted, plaintext);

    // Test ChaCha20-Poly1305
    let key = CryptoKey::generate(Algorithm::ChaCha20Poly1305).expect("Failed to generate key");
    let ciphertext = encrypt(&key, plaintext, aad).expect("Encryption failed");
    let decrypted = decrypt(&key, &ciphertext, aad).expect("Decryption failed");
    assert_eq!(decrypted, plaintext);

    // Test key derivation
    let password = b"test-password-123";
    let salt = generate_salt().expect("Failed to generate salt");
    let derived_key =
        CryptoKey::derive_from_password(password, &salt, 100_000, Algorithm::Aes256Gcm)
            .expect("Key derivation failed");
    assert_eq!(derived_key.key_len(), 32);
}

fn test_asymmetric_keys() {
    use airgap_sync::keys::*;

    // Test RSA key generation and signing
    let rsa_key =
        AsymmetricKey::generate(AsymmetricAlgorithm::Rsa2048).expect("Failed to generate RSA key");

    let message = b"Test message for RSA signature";
    let signature = rsa_key.sign(message).expect("Failed to sign");
    rsa_key
        .verify(message, &signature)
        .expect("Failed to verify");

    // Test ECDSA key generation and signing
    let ecdsa_key = AsymmetricKey::generate(AsymmetricAlgorithm::EcdsaP256)
        .expect("Failed to generate ECDSA key");

    let signature = ecdsa_key.sign(message).expect("Failed to sign");
    ecdsa_key
        .verify(message, &signature)
        .expect("Failed to verify");

    // Test public key export
    let pem = ecdsa_key.public_key_pem();
    assert!(pem.contains("BEGIN PUBLIC KEY"));

    // Test ECDH key agreement
    let key1 = AsymmetricKey::generate(AsymmetricAlgorithm::EcdsaP256).unwrap();
    let key2 = AsymmetricKey::generate(AsymmetricAlgorithm::EcdsaP256).unwrap();

    let agreement1 = KeyAgreement::from_key(&key1).unwrap();
    let agreement2 = KeyAgreement::from_key(&key2).unwrap();

    let shared1 = agreement1.agree(key2.public_key_bytes()).unwrap();
    let shared2 = agreement2.agree(key1.public_key_bytes()).unwrap();

    // Both sides should derive the same shared secret
    assert_eq!(shared1, shared2);
}

#[test]
fn test_error_handling() {
    use airgap_sync::crypto::*;

    // Test decryption with wrong key
    let key1 = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
    let key2 = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();

    let plaintext = b"Secret data";
    let ciphertext = encrypt(&key1, plaintext, b"").unwrap();

    // Should fail with wrong key
    assert!(decrypt(&key2, &ciphertext, b"").is_err());

    // Test invalid key length
    let bad_key = CryptoKey::new(vec![0u8; 16], Algorithm::Aes256Gcm);
    assert!(bad_key.is_err());
}

#[test]
fn test_config_validation() {
    use airgap_sync::config::*;

    // Test invalid config - no devices
    let mut config = Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: PathBuf::from("/tmp"),
            exclude: vec![],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![],
        policy: PolicyConfig::default(),
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    assert!(config.validate().is_err());

    // Add device but with duplicate ID
    config.device.push(DeviceConfig {
        id: "USB001".to_string(),
        name: "Device 1".to_string(),
        mount_point: PathBuf::from("/mnt/usb1"),
        encryption: EncryptionConfig::default(),
    });

    config.device.push(DeviceConfig {
        id: "USB001".to_string(), // Duplicate ID
        name: "Device 2".to_string(),
        mount_point: PathBuf::from("/mnt/usb2"),
        encryption: EncryptionConfig::default(),
    });

    assert!(config.validate().is_err());
}
