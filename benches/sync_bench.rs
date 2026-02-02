//! Sync engine benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use airgap_sync::{sync::*, diff::*, chunk::*, config::*};
use tempfile::TempDir;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn create_test_files(dir: &Path, count: usize, size: usize) {
    for i in 0..count {
        let path = dir.join(format!("file_{}.dat", i));
        let mut file = File::create(path).unwrap();
        let data = vec![i as u8; size];
        file.write_all(&data).unwrap();
    }
}

fn bench_diff_engine(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_engine");
    
    for file_count in [100, 1000, 10000].iter() {
        group.bench_function(format!("diff_{}_files", file_count), |b| {
            let temp_dir = TempDir::new().unwrap();
            create_test_files(temp_dir.path(), *file_count, 1024);
            
            let mut diff_engine = DiffEngine::new();
            
            b.iter(|| {
                let changes = diff_engine.compute_full_diff(
                    temp_dir.path(),
                    None,
                    &[],
                ).unwrap();
                black_box(changes);
            });
        });
    }
    
    group.finish();
}

fn bench_chunk_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_processing");
    
    for file_size in [1024 * 1024, 10 * 1024 * 1024, 100 * 1024 * 1024].iter() {
        group.throughput(Throughput::Bytes(*file_size as u64));
        group.bench_function(format!("chunk_{}_mb", file_size / (1024 * 1024)), |b| {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("large_file.dat");
            
            // Create test file
            let mut file = File::create(&file_path).unwrap();
            let chunk = vec![0u8; 1024 * 1024];
            for _ in 0..(*file_size / (1024 * 1024)) {
                file.write_all(&chunk).unwrap();
            }
            file.sync_all().unwrap();
            
            let processor = ChunkProcessor::new();
            let key = airgap_sync::crypto::CryptoKey::generate(
                airgap_sync::crypto::Algorithm::Aes256Gcm
            ).unwrap();
            
            b.iter(|| {
                let chunks = processor.process_file(
                    &file_path,
                    1024 * 1024, // 1MB chunks
                    6,           // Compression level
                    &key,
                ).unwrap();
                black_box(chunks);
            });
        });
    }
    
    group.finish();
}

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");
    
    let data_sizes = vec![
        ("1KB", vec![42u8; 1024]),
        ("100KB", vec![42u8; 100 * 1024]),
        ("1MB", vec![42u8; 1024 * 1024]),
    ];
    
    for (name, data) in data_sizes {
        for level in [0, 3, 6, 9].iter() {
            group.throughput(Throughput::Bytes(data.len() as u64));
            group.bench_function(format!("zstd_{}_{}_level_{}", name, data.len(), level), |b| {
                b.iter(|| {
                    use zstd::stream::encode_all;
                    let compressed = encode_all(&data[..], *level).unwrap();
                    black_box(compressed);
                });
            });
        }
    }
    
    group.finish();
}

fn bench_hash_computation(c: &mut Criterion) {
    use sha2::{Sha256, Digest};
    
    let mut group = c.benchmark_group("hash_computation");
    
    for size in [1024, 1024 * 1024, 10 * 1024 * 1024].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_function(format!("sha256_{}_bytes", size), |b| {
            let data = vec![0u8; *size];
            
            b.iter(|| {
                let mut hasher = Sha256::new();
                hasher.update(&data);
                let hash = hasher.finalize();
                black_box(hash);
            });
        });
    }
    
    group.finish();
}

fn bench_snapshot_operations(c: &mut Criterion) {
    use airgap_sync::snapshot::*;
    use airgap_sync::diff::FileChange;
    use std::path::PathBuf;
    use chrono::Utc;
    
    let mut group = c.benchmark_group("snapshot_operations");
    
    group.bench_function("snapshot_creation", |b| {
        let changes: Vec<FileChange> = (0..1000)
            .map(|i| FileChange::Added {
                path: PathBuf::from(format!("file_{}.txt", i)),
                size: 1024,
                modified: Utc::now(),
                hash: format!("hash_{}", i),
                permissions: 0o644,
            })
            .collect();
        
        b.iter(|| {
            let snapshot = Snapshot::new(
                "DEVICE_001".to_string(),
                changes.clone(),
                1000,
                1024 * 1000,
            );
            black_box(snapshot);
        });
    });
    
    group.bench_function("snapshot_save_load", |b| {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SnapshotManager::new(temp_dir.path()).unwrap();
        
        let snapshot = Snapshot::new(
            "DEVICE_001".to_string(),
            vec![],
            0,
            0,
        );
        
        b.iter(|| {
            manager.save_snapshot(&snapshot).unwrap();
            let loaded = manager.load_snapshot("DEVICE_001", &snapshot.id).unwrap();
            black_box(loaded);
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_diff_engine,
    bench_chunk_processing,
    bench_compression,
    bench_hash_computation,
    bench_snapshot_operations
);
criterion_main!(benches);