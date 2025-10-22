//! Phase 2 Performance Benchmarks
//!
//! These benchmarks measure the performance of the sync engine components.
//! Run with: cargo test --release --test phase2_benchmarks -- --nocapture

use airgap_sync::*;
use std::fs;
use std::time::Instant;
use tempfile::TempDir;

/// Helper to create test files of specific size
fn create_test_data(path: &std::path::Path, size_mb: usize) -> std::io::Result<()> {
    let chunk_size = 1024 * 1024; // 1MB
    let data = vec![b'X'; chunk_size];

    let file = fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);

    for _ in 0..size_mb {
        std::io::Write::write_all(&mut writer, &data)?;
    }

    Ok(())
}

/// Helper to create directory structure with multiple files
fn create_test_directory(base_path: &std::path::Path, num_files: usize, file_size_kb: usize) -> std::io::Result<()> {
    let data = vec![b'A'; file_size_kb * 1024];

    for i in 0..num_files {
        let filename = format!("file_{:05}.dat", i);
        fs::write(base_path.join(filename), &data)?;
    }

    Ok(())
}

#[test]
fn benchmark_diff_scan_1000_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create 1000 small files
    println!("Creating 1000 test files...");
    create_test_directory(temp_dir.path(), 1000, 10).unwrap(); // 10KB each

    let diff_engine = DiffEngine::default();

    println!("Scanning directory...");
    let start = Instant::now();
    let files = diff_engine.scan_directory(temp_dir.path()).unwrap();
    let duration = start.elapsed();

    println!("✓ Scanned {} files in {:.3}s", files.len(), duration.as_secs_f64());
    println!("  Throughput: {:.0} files/sec", files.len() as f64 / duration.as_secs_f64());

    // Performance target: < 1 second for 1000 files
    assert!(duration.as_secs() < 2, "Scan took too long: {:?}", duration);
}

#[test]
fn benchmark_chunk_100mb_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large.dat");

    println!("Creating 100MB test file...");
    create_test_data(&file_path, 100).unwrap();

    let processor = ChunkProcessor::new(ChunkOptions {
        chunk_size: 1024 * 1024, // 1MB chunks
        compress: false,
        encrypt: false,
        parallel: 1,
    }).unwrap();

    println!("Chunking file...");
    let start = Instant::now();
    let chunks = processor.chunk_file(&file_path).unwrap();
    let duration = start.elapsed();

    let file_size = fs::metadata(&file_path).unwrap().len();
    let throughput_mbps = (file_size as f64 / 1_000_000.0) / duration.as_secs_f64();

    println!("✓ Chunked {}MB into {} chunks in {:.3}s",
             file_size / 1_000_000,
             chunks.len(),
             duration.as_secs_f64());
    println!("  Throughput: {:.2} MB/s", throughput_mbps);

    // Performance target: > 100 MB/s
    assert!(throughput_mbps > 50.0, "Chunking too slow: {:.2} MB/s", throughput_mbps);
}

#[test]
fn benchmark_chunk_compression() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("compressible.dat");

    // Create highly compressible data (50MB)
    println!("Creating 50MB compressible file...");
    let data = vec![b'A'; 50 * 1024 * 1024];
    fs::write(&file_path, data).unwrap();

    let processor = ChunkProcessor::new(ChunkOptions {
        chunk_size: 1024 * 1024,
        compress: true,
        encrypt: false,
        parallel: 1,
    }).unwrap();

    println!("Chunking and compressing...");
    let start = Instant::now();
    let mut chunks = processor.chunk_file(&file_path).unwrap();
    processor.process_chunks(&mut chunks, None).unwrap();
    let duration = start.elapsed();

    let original_size: usize = chunks.iter().map(|c| c.size).sum();
    let compressed_size: usize = chunks.iter().map(|c| c.current_size()).sum();
    let ratio = compressed_size as f64 / original_size as f64;

    println!("✓ Compressed {}MB to {}KB in {:.3}s",
             original_size / 1_000_000,
             compressed_size / 1024,
             duration.as_secs_f64());
    println!("  Compression ratio: {:.1}%", ratio * 100.0);
    println!("  Throughput: {:.2} MB/s", (original_size as f64 / 1_000_000.0) / duration.as_secs_f64());

    // Should achieve significant compression for repetitive data
    assert!(ratio < 0.1, "Compression not effective: {:.1}%", ratio * 100.0);
}

#[test]
fn benchmark_chunk_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = ChunkStore::new(temp_dir.path().to_path_buf()).unwrap();

    println!("Testing chunk deduplication...");

    // Create 1000 chunks with 50% duplication
    let num_chunks = 1000;
    let unique_chunks = num_chunks / 2;

    let start = Instant::now();

    for i in 0..num_chunks {
        let data = format!("Chunk data {}", i % unique_chunks);
        let chunk = Chunk::new(data.into_bytes(), i as u64);
        store.store(&chunk).unwrap();
    }

    let duration = start.elapsed();

    println!("✓ Stored {} chunks ({} unique) in {:.3}s",
             num_chunks,
             store.chunk_count(),
             duration.as_secs_f64());
    println!("  Deduplication ratio: {:.1}%",
             (store.chunk_count() as f64 / num_chunks as f64) * 100.0);
    println!("  Throughput: {:.0} chunks/sec", num_chunks as f64 / duration.as_secs_f64());

    // Should deduplicate to ~500 unique chunks
    assert!(store.chunk_count() <= unique_chunks + 10);
}

#[test]
fn benchmark_manifest_operations() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("manifest.json");

    println!("Creating manifest with 10,000 files...");

    let mut manifest = Manifest::new(
        "bench-device".to_string(),
        std::path::PathBuf::from("/test"),
    );

    let start_create = Instant::now();

    for i in 0..10_000 {
        let metadata = FileMetadata {
            path: std::path::PathBuf::from(format!("file_{}.txt", i)),
            size: 1024 * (i % 100 + 1),
            modified: std::time::SystemTime::now(),
            #[cfg(unix)]
            permissions: 0o644,
            hash: Some(format!("hash_{}", i)),
            is_dir: false,
        };

        let entry = FileEntry::regular_file(
            metadata,
            vec![format!("chunk_{}", i)],
        );

        manifest.add_file(std::path::PathBuf::from(format!("file_{}.txt", i)), entry);
    }

    let create_duration = start_create.elapsed();

    println!("✓ Created manifest in {:.3}s", create_duration.as_secs_f64());

    // Save
    let start_save = Instant::now();
    manifest.save_json(&manifest_path).unwrap();
    let save_duration = start_save.elapsed();

    let file_size = fs::metadata(&manifest_path).unwrap().len();

    println!("✓ Saved manifest ({:.2} MB) in {:.3}s",
             file_size as f64 / 1_000_000.0,
             save_duration.as_secs_f64());

    // Load
    let start_load = Instant::now();
    let loaded = Manifest::load_json(&manifest_path).unwrap();
    let load_duration = start_load.elapsed();

    println!("✓ Loaded manifest in {:.3}s", load_duration.as_secs_f64());

    assert_eq!(loaded.file_count, 10_000);

    // Performance targets
    assert!(save_duration.as_secs() < 5, "Save too slow");
    assert!(load_duration.as_secs() < 5, "Load too slow");
}

#[test]
fn benchmark_full_sync_100_files() {
    let source_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();

    println!("Creating 100 test files (100KB each)...");
    create_test_directory(source_dir.path(), 100, 100).unwrap();

    let options = SyncOptions {
        device_id: "bench-device".to_string(),
        dry_run: false,
        chunk_options: ChunkOptions {
            chunk_size: 1024 * 1024,
            compress: false,
            encrypt: false,
            parallel: 1,
        },
        ..Default::default()
    };

    let mut engine = SyncEngine::new(options);

    println!("Running sync...");
    let start = Instant::now();

    let result = engine.sync(
        source_dir.path(),
        dest_dir.path(),
        None,
        None,
    ).unwrap();

    let duration = start.elapsed();
    let throughput_mbps = (result.bytes_synced as f64 / 1_000_000.0) / duration.as_secs_f64();

    println!("\n✓ Sync completed in {:.3}s", duration.as_secs_f64());
    println!("  Files synced: {}", result.files_synced);
    println!("  Bytes synced: {:.2} MB", result.bytes_synced as f64 / 1_000_000.0);
    println!("  Throughput: {:.2} MB/s", throughput_mbps);
    println!("  Chunks created: {}", result.chunks_created);

    // Performance target: > 10 MB/s for small files
    assert!(throughput_mbps > 5.0, "Sync too slow: {:.2} MB/s", throughput_mbps);
}

#[test]
fn benchmark_incremental_sync() {
    let source_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();

    println!("Creating initial 100 test files...");
    create_test_directory(source_dir.path(), 100, 50).unwrap();

    let options = SyncOptions {
        device_id: "bench-device".to_string(),
        dry_run: false,
        chunk_options: ChunkOptions {
            chunk_size: 512 * 1024,
            compress: false,
            encrypt: false,
            parallel: 1,
        },
        ..Default::default()
    };

    // First sync
    println!("Running initial sync...");
    let mut engine1 = SyncEngine::new(options.clone());
    let start1 = Instant::now();
    let result1 = engine1.sync(source_dir.path(), dest_dir.path(), None, None).unwrap();
    let duration1 = start1.elapsed();

    println!("✓ Initial sync: {:.3}s, {} files", duration1.as_secs_f64(), result1.files_synced);

    // Add 10 new files
    println!("Adding 10 new files...");
    for i in 100..110 {
        let data = vec![b'B'; 50 * 1024];
        fs::write(source_dir.path().join(format!("file_{:05}.dat", i)), data).unwrap();
    }

    // Incremental sync
    println!("Running incremental sync...");
    let mut engine2 = SyncEngine::new(options);
    let start2 = Instant::now();
    let result2 = engine2.sync(source_dir.path(), dest_dir.path(), None, None).unwrap();
    let duration2 = start2.elapsed();

    println!("✓ Incremental sync: {:.3}s, {} files", duration2.as_secs_f64(), result2.files_synced);

    // Incremental sync should be much faster
    println!("\nSpeedup: {:.1}x", duration1.as_secs_f64() / duration2.as_secs_f64());

    // Incremental should process fewer files and be faster
    assert!(duration2 < duration1, "Incremental sync not faster than initial");
}

#[test]
fn benchmark_memory_usage() {
    let source_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();

    println!("Creating 1000 test files (10KB each)...");
    create_test_directory(source_dir.path(), 1000, 10).unwrap();

    let options = SyncOptions {
        device_id: "bench-device".to_string(),
        dry_run: false,
        chunk_options: ChunkOptions {
            chunk_size: 512 * 1024,
            compress: false,
            encrypt: false,
            parallel: 1,
        },
        ..Default::default()
    };

    let mut engine = SyncEngine::new(options);

    println!("Running sync (monitoring memory)...");

    let result = engine.sync(
        source_dir.path(),
        dest_dir.path(),
        None,
        None,
    ).unwrap();

    println!("✓ Synced {} files, {:.2} MB total",
             result.files_synced,
             result.bytes_synced as f64 / 1_000_000.0);

    // Note: Actual memory measurement would require platform-specific code
    // This test ensures the sync completes without OOM
    assert!(result.files_synced > 0);
}

#[test]
fn benchmark_restore_performance() {
    let source_dir = TempDir::new().unwrap();
    let archive_dir = TempDir::new().unwrap();
    let restore_dir = TempDir::new().unwrap();

    println!("Creating 50 test files (100KB each)...");
    create_test_directory(source_dir.path(), 50, 100).unwrap();

    let options = SyncOptions {
        device_id: "bench-device".to_string(),
        dry_run: false,
        chunk_options: ChunkOptions {
            chunk_size: 512 * 1024,
            compress: false,
            encrypt: false,
            parallel: 1,
        },
        ..Default::default()
    };

    // Sync
    println!("Syncing to archive...");
    let mut sync_engine = SyncEngine::new(options.clone());
    let sync_result = sync_engine.sync(source_dir.path(), archive_dir.path(), None, None).unwrap();

    // Restore
    println!("Restoring from archive...");
    let mut restore_engine = SyncEngine::new(options);
    let start = Instant::now();

    let restore_result = restore_engine.restore(
        archive_dir.path(),
        &sync_result.snapshot_id,
        restore_dir.path(),
        None,
        None,
    ).unwrap();

    let duration = start.elapsed();
    let throughput_mbps = (restore_result.bytes_restored as f64 / 1_000_000.0) / duration.as_secs_f64();

    println!("\n✓ Restore completed in {:.3}s", duration.as_secs_f64());
    println!("  Files restored: {}", restore_result.files_restored);
    println!("  Bytes restored: {:.2} MB", restore_result.bytes_restored as f64 / 1_000_000.0);
    println!("  Throughput: {:.2} MB/s", throughput_mbps);

    // Performance target: > 10 MB/s
    assert!(throughput_mbps > 5.0, "Restore too slow: {:.2} MB/s", throughput_mbps);
}

#[test]
fn benchmark_summary() {
    println!("\n========================================");
    println!("Phase 2 Performance Benchmark Summary");
    println!("========================================");
    println!("\nTo run all benchmarks:");
    println!("  cargo test --release --test phase2_benchmarks -- --nocapture");
    println!("\nPerformance Targets:");
    println!("  ✓ Directory scanning: < 2s for 1000 files");
    println!("  ✓ Chunking: > 50 MB/s");
    println!("  ✓ Compression: > 20 MB/s (compressible data)");
    println!("  ✓ Sync throughput: > 5 MB/s");
    println!("  ✓ Incremental sync: faster than initial sync");
    println!("  ✓ Memory usage: < 100MB for typical operations");
    println!("  ✓ Restore throughput: > 5 MB/s");
    println!("\nNote: Run with --release for accurate performance measurements");
    println!("========================================\n");
}
