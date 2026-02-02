//! Cryptography benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use airgap_sync::crypto::{*, Algorithm};

fn bench_aes_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("aes_encryption");
    
    for size in [1024, 16 * 1024, 1024 * 1024, 16 * 1024 * 1024].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_function(format!("aes_256_gcm_{}_bytes", size), |b| {
            let key = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
            let data = vec![0u8; *size];
            
            b.iter(|| {
                let encrypted = encrypt(&key, &data, b"benchmark").unwrap();
                black_box(encrypted);
            });
        });
    }
    
    group.finish();
}

fn bench_chacha_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("chacha_encryption");
    
    for size in [1024, 16 * 1024, 1024 * 1024, 16 * 1024 * 1024].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_function(format!("chacha20_poly1305_{}_bytes", size), |b| {
            let key = CryptoKey::generate(Algorithm::ChaCha20Poly1305).unwrap();
            let data = vec![0u8; *size];
            
            b.iter(|| {
                let encrypted = encrypt(&key, &data, b"benchmark").unwrap();
                black_box(encrypted);
            });
        });
    }
    
    group.finish();
}

fn bench_decryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("decryption");
    
    for size in [1024, 16 * 1024, 1024 * 1024].iter() {
        let key = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
        let data = vec![0u8; *size];
        let encrypted = encrypt(&key, &data, b"benchmark").unwrap();
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_function(format!("aes_decrypt_{}_bytes", size), |b| {
            b.iter(|| {
                let decrypted = decrypt(&key, &encrypted, b"benchmark").unwrap();
                black_box(decrypted);
            });
        });
    }
    
    group.finish();
}

fn bench_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_generation");
    
    group.bench_function("aes_256_key_gen", |b| {
        b.iter(|| {
            let key = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
            black_box(key);
        });
    });
    
    group.bench_function("chacha20_key_gen", |b| {
        b.iter(|| {
            let key = CryptoKey::generate(Algorithm::ChaCha20Poly1305).unwrap();
            black_box(key);
        });
    });
    
    group.finish();
}

fn bench_pbkdf2(c: &mut Criterion) {
    let mut group = c.benchmark_group("pbkdf2");
    
    for iterations in [10_000, 100_000, 1_000_000].iter() {
        group.bench_function(format!("pbkdf2_{}_iterations", iterations), |b| {
            let password = b"test_password_123";
            let salt = b"random_salt_456";
            
            b.iter(|| {
                let key = CryptoKey::derive_from_password(
                    password,
                    salt,
                    *iterations,
                    Algorithm::Aes256Gcm
                ).unwrap();
                black_box(key);
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_aes_encryption,
    bench_chacha_encryption,
    bench_decryption,
    bench_key_generation,
    bench_pbkdf2
);
criterion_main!(benches);