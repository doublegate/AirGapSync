//! Phase 2 Integration Tests
//!
//! These tests validate the complete sync engine functionality including
//! diff, chunk, manifest, archive, and sync modules working together.

use airgap_sync::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test directory structure with files
fn create_test_files(base_path: &std::path::Path) -> std::io::Result<()> {
    // Create some test files
    fs::write(base_path.join("file1.txt"), b"Hello, world!")?;
    fs::write(base_path.join("file2.txt"), b"Another file with different content.")?;

    // Create subdirectory
    fs::create_dir(base_path.join("subdir"))?;
    fs::write(base_path.join("subdir/file3.txt"), b"File in subdirectory.")?;

    // Create a larger file for chunking tests
    let large_content = vec![b'X'; 2 * 1024 * 1024]; // 2MB
    fs::write(base_path.join("large_file.bin"), large_content)?;

    Ok(())
}

#[test]
fn test_diff_engine_basic() {
    let source_dir = TempDir::new().unwrap();
    create_test_files(source_dir.path()).unwrap();

    let diff_engine = DiffEngine::default();

    // Scan the directory
    let files = diff_engine.scan_directory(source_dir.path()).unwrap();

    // Should find 4 files + 1 directory
    assert!(files.len() >= 4);
    assert!(files.contains_key(&PathBuf::from("file1.txt")));
    assert!(files.contains_key(&PathBuf::from("file2.txt")));
    assert!(files.contains_key(&PathBuf::from("subdir/file3.txt")));
    assert!(files.contains_key(&PathBuf::from("large_file.bin")));
}

#[test]
fn test_diff_detect_changes() {
    let source_dir = TempDir::new().unwrap();
    create_test_files(source_dir.path()).unwrap();

    let diff_engine = DiffEngine::default();

    // First scan
    let initial_files = diff_engine.scan_directory(source_dir.path()).unwrap();

    // Modify a file
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(source_dir.path().join("file1.txt"), b"Modified content!").unwrap();

    // Diff against initial state
    let changes = diff_engine.diff(source_dir.path(), &initial_files).unwrap();

    // Should detect the modification
    let modified_count = changes.iter()
        .filter(|c| c.change_type == ChangeType::Modified)
        .count();

    assert!(modified_count > 0);
}

#[test]
fn test_chunk_processor() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.bin");

    // Create a 2.5MB file
    let data = vec![b'A'; 2_500_000];
    fs::write(&file_path, &data).unwrap();

    let processor = ChunkProcessor::new(ChunkOptions {
        chunk_size: 1024 * 1024, // 1MB chunks
        compress: false,
        encrypt: false,
        parallel: 1,
    }).unwrap();

    // Chunk the file
    let chunks = processor.chunk_file(&file_path).unwrap();

    // Should create 3 chunks (2 full + 1 partial)
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].size, 1024 * 1024);
    assert_eq!(chunks[1].size, 1024 * 1024);
    assert_eq!(chunks[2].size, 2_500_000 - 2 * 1024 * 1024);

    // Each chunk should have a unique hash
    assert_ne!(chunks[0].hash, chunks[1].hash);
    assert_ne!(chunks[1].hash, chunks[2].hash);
}

#[test]
fn test_chunk_compression() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("compressible.txt");

    // Create highly compressible data
    let data = vec![b'A'; 100_000];
    fs::write(&file_path, &data).unwrap();

    let processor = ChunkProcessor::new(ChunkOptions {
        chunk_size: 50_000,
        compress: true,
        encrypt: false,
        parallel: 1,
    }).unwrap();

    let mut chunks = processor.chunk_file(&file_path).unwrap();
    processor.process_chunks(&mut chunks, None).unwrap();

    // Compression should reduce size significantly for repetitive data
    assert!(chunks[0].compressed);
    assert!(chunks[0].compressed_size < chunks[0].size);
}

#[test]
fn test_chunk_reassembly() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    let output_path = temp_dir.path().join("output.txt");

    let original_data = b"This is a test file with some content that will be chunked and reassembled.";
    fs::write(&input_path, original_data).unwrap();

    let processor = ChunkProcessor::new(ChunkOptions {
        chunk_size: 20, // Small chunks for testing
        compress: false,
        encrypt: false,
        parallel: 1,
    }).unwrap();

    // Chunk and reassemble
    let mut chunks = processor.chunk_file(&input_path).unwrap();
    processor.reassemble_chunks(&mut chunks, &output_path, None).unwrap();

    // Verify content matches
    let reassembled = fs::read(&output_path).unwrap();
    assert_eq!(reassembled, original_data);
}

#[test]
fn test_chunk_store_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = ChunkStore::new(temp_dir.path().to_path_buf()).unwrap();

    // Create two identical chunks
    let chunk1 = Chunk::new(b"identical content".to_vec(), 0);
    let chunk2 = Chunk::new(b"identical content".to_vec(), 100);

    // First store should succeed
    let stored1 = store.store(&chunk1).unwrap();
    assert!(stored1);

    // Second store should deduplicate
    let stored2 = store.store(&chunk2).unwrap();
    assert!(!stored2);

    // Should only have 1 unique chunk
    assert_eq!(store.chunk_count(), 1);

    // Create a different chunk
    let chunk3 = Chunk::new(b"different content".to_vec(), 200);
    let stored3 = store.store(&chunk3).unwrap();
    assert!(stored3);

    assert_eq!(store.chunk_count(), 2);
}

#[test]
fn test_manifest_creation() {
    let manifest = Manifest::new(
        "test-device".to_string(),
        PathBuf::from("/test/source"),
    );

    assert_eq!(manifest.device_id, "test-device");
    assert_eq!(manifest.file_count, 0);
    assert_eq!(manifest.total_bytes, 0);
    assert!(!manifest.snapshot_id.is_empty());
}

#[test]
fn test_manifest_add_remove_files() {
    let mut manifest = Manifest::new(
        "test-device".to_string(),
        PathBuf::from("/test/source"),
    );

    let metadata = FileMetadata {
        path: PathBuf::from("test.txt"),
        size: 1000,
        modified: std::time::SystemTime::now(),
        #[cfg(unix)]
        permissions: 0o644,
        hash: None,
        is_dir: false,
    };

    let entry = FileEntry::regular_file(
        metadata.clone(),
        vec!["chunk1".to_string(), "chunk2".to_string()],
    );

    manifest.add_file(PathBuf::from("test.txt"), entry);

    assert_eq!(manifest.file_count, 1);
    assert_eq!(manifest.total_bytes, 1000);
    assert_eq!(manifest.chunk_count, 2);

    manifest.remove_file(&PathBuf::from("test.txt"));

    assert_eq!(manifest.file_count, 0);
    assert_eq!(manifest.total_bytes, 0);
    assert_eq!(manifest.chunk_count, 0);
}

#[test]
fn test_manifest_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("manifest.json");

    let mut manifest = Manifest::new(
        "test-device".to_string(),
        PathBuf::from("/test/source"),
    );

    let metadata = FileMetadata {
        path: PathBuf::from("file.txt"),
        size: 500,
        modified: std::time::SystemTime::now(),
        #[cfg(unix)]
        permissions: 0o644,
        hash: Some("abc123".to_string()),
        is_dir: false,
    };

    let entry = FileEntry::regular_file(metadata, vec!["chunk1".to_string()]);
    manifest.add_file(PathBuf::from("file.txt"), entry);

    // Save
    manifest.save_json(&manifest_path).unwrap();

    // Load
    let loaded = Manifest::load_json(&manifest_path).unwrap();

    assert_eq!(loaded.snapshot_id, manifest.snapshot_id);
    assert_eq!(loaded.file_count, 1);
    assert!(loaded.has_file(&PathBuf::from("file.txt")));
}

#[test]
fn test_archive_initialize() {
    let temp_dir = TempDir::new().unwrap();

    let archive = Archive::initialize(
        temp_dir.path().to_path_buf(),
        "test-device".to_string(),
    ).unwrap();

    // Verify structure was created
    let archive_path = temp_dir.path().join(".airgapsync");
    assert!(archive_path.exists());

    let device_path = archive_path.join("device-test-device");
    assert!(device_path.exists());

    let snapshots_path = device_path.join("snapshots");
    assert!(snapshots_path.exists());
}

#[test]
fn test_archive_create_snapshot() {
    let temp_dir = TempDir::new().unwrap();
    let mut archive = Archive::initialize(
        temp_dir.path().to_path_buf(),
        "test-device".to_string(),
    ).unwrap();

    let manifest = Manifest::new(
        "test-device".to_string(),
        PathBuf::from("/test/source"),
    );

    let snapshot_id = archive.create_snapshot(&manifest).unwrap();

    // Verify snapshot was created
    let loaded_manifest = archive.load_manifest(&snapshot_id).unwrap();
    assert_eq!(loaded_manifest.snapshot_id, snapshot_id);
}

#[test]
fn test_archive_store_retrieve_chunk() {
    let temp_dir = TempDir::new().unwrap();
    let mut archive = Archive::initialize(
        temp_dir.path().to_path_buf(),
        "test-device".to_string(),
    ).unwrap();

    let manifest = Manifest::new(
        "test-device".to_string(),
        PathBuf::from("/test/source"),
    );

    let snapshot_id = archive.create_snapshot(&manifest).unwrap();

    // Store a chunk
    let chunk = Chunk::new(b"test chunk data".to_vec(), 0);
    let chunk_id = chunk.id().to_string();

    archive.store_chunk(&chunk).unwrap();

    // Retrieve the chunk
    let retrieved = archive.retrieve_chunk(&snapshot_id, &chunk_id).unwrap();
    assert_eq!(retrieved.data, chunk.data);
}

#[test]
fn test_full_sync_workflow_dry_run() {
    let source_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();

    create_test_files(source_dir.path()).unwrap();

    let options = SyncOptions {
        device_id: "test-device".to_string(),
        dry_run: true,
        ..Default::default()
    };

    let mut engine = SyncEngine::new(options);

    let result = engine.sync(
        source_dir.path(),
        dest_dir.path(),
        None,
        None,
    ).unwrap();

    assert_eq!(result.snapshot_id, "dry-run");
    assert!(result.files_synced > 0);
}

#[test]
fn test_full_sync_workflow_no_encryption() {
    let source_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();

    create_test_files(source_dir.path()).unwrap();

    let options = SyncOptions {
        device_id: "test-device".to_string(),
        dry_run: false,
        chunk_options: ChunkOptions {
            chunk_size: 512 * 1024, // 512KB chunks
            compress: true,
            encrypt: false, // No encryption for test
            parallel: 1,
        },
        ..Default::default()
    };

    let mut engine = SyncEngine::new(options);

    let result = engine.sync(
        source_dir.path(),
        dest_dir.path(),
        None, // No encryption key
        None,
    ).unwrap();

    assert_ne!(result.snapshot_id, "dry-run");
    assert!(result.files_synced > 0);
    assert!(result.bytes_synced > 0);
    assert!(result.chunks_created > 0);
}

#[test]
fn test_incremental_sync() {
    let source_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();

    create_test_files(source_dir.path()).unwrap();

    let options = SyncOptions {
        device_id: "test-device".to_string(),
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
    let mut engine1 = SyncEngine::new(options.clone());
    let result1 = engine1.sync(
        source_dir.path(),
        dest_dir.path(),
        None,
        None,
    ).unwrap();

    let first_files_synced = result1.files_synced;

    // Add a new file
    fs::write(source_dir.path().join("new_file.txt"), b"New content").unwrap();

    // Second sync (incremental)
    let mut engine2 = SyncEngine::new(options);
    let result2 = engine2.sync(
        source_dir.path(),
        dest_dir.path(),
        None,
        None,
    ).unwrap();

    // Should sync the new file plus possibly unchanged files
    assert!(result2.files_synced >= 1);
}

#[test]
fn test_archive_verification() {
    let temp_dir = TempDir::new().unwrap();
    let mut archive = Archive::initialize(
        temp_dir.path().to_path_buf(),
        "test-device".to_string(),
    ).unwrap();

    let mut manifest = Manifest::new(
        "test-device".to_string(),
        PathBuf::from("/test/source"),
    );

    // Add a file with chunks
    let metadata = FileMetadata {
        path: PathBuf::from("test.txt"),
        size: 100,
        modified: std::time::SystemTime::now(),
        #[cfg(unix)]
        permissions: 0o644,
        hash: None,
        is_dir: false,
    };

    let chunk = Chunk::new(b"test content".to_vec(), 0);
    let chunk_id = chunk.id().to_string();

    let entry = FileEntry::regular_file(metadata, vec![chunk_id.clone()]);
    manifest.add_file(PathBuf::from("test.txt"), entry);

    let snapshot_id = archive.create_snapshot(&manifest).unwrap();
    archive.store_chunk(&chunk).unwrap();

    // Verify the archive
    let result = archive.verify(&snapshot_id).unwrap();

    assert!(result.is_valid);
    assert_eq!(result.verified_files, 1);
    assert_eq!(result.verified_chunks, 1);
    assert!(result.missing_chunks.is_empty());
}

#[test]
fn test_archive_statistics() {
    let temp_dir = TempDir::new().unwrap();
    let mut archive = Archive::initialize(
        temp_dir.path().to_path_buf(),
        "test-device".to_string(),
    ).unwrap();

    // Create a snapshot
    let mut manifest = Manifest::new(
        "test-device".to_string(),
        PathBuf::from("/test/source"),
    );

    manifest.total_bytes = 10_000;
    manifest.compressed_bytes = 8_000;
    manifest.file_count = 5;

    archive.create_snapshot(&manifest).unwrap();

    let stats = archive.statistics().unwrap();

    assert_eq!(stats.snapshot_count, 1);
    assert_eq!(stats.total_files, 5);
    assert_eq!(stats.total_bytes, 10_000);
    assert_eq!(stats.compressed_bytes, 8_000);
}

#[test]
fn test_restore_workflow() {
    let source_dir = TempDir::new().unwrap();
    let archive_dir = TempDir::new().unwrap();
    let restore_dir = TempDir::new().unwrap();

    create_test_files(source_dir.path()).unwrap();

    // Sync files to archive
    let sync_options = SyncOptions {
        device_id: "test-device".to_string(),
        dry_run: false,
        chunk_options: ChunkOptions {
            chunk_size: 512 * 1024,
            compress: false,
            encrypt: false,
            parallel: 1,
        },
        ..Default::default()
    };

    let mut sync_engine = SyncEngine::new(sync_options.clone());
    let sync_result = sync_engine.sync(
        source_dir.path(),
        archive_dir.path(),
        None,
        None,
    ).unwrap();

    // Restore from archive
    let mut restore_engine = SyncEngine::new(sync_options);
    let restore_result = restore_engine.restore(
        archive_dir.path(),
        &sync_result.snapshot_id,
        restore_dir.path(),
        None,
        None,
    ).unwrap();

    assert!(restore_result.files_restored > 0);

    // Verify restored files match originals
    let original_content = fs::read(source_dir.path().join("file1.txt")).unwrap();
    let restored_content = fs::read(restore_dir.path().join("file1.txt")).unwrap();
    assert_eq!(original_content, restored_content);
}

#[test]
fn test_change_summary() {
    let changes = vec![
        FileChange::new(
            PathBuf::from("added.txt"),
            ChangeType::Added,
            Some(FileMetadata {
                path: PathBuf::from("added.txt"),
                size: 100,
                modified: std::time::SystemTime::now(),
                #[cfg(unix)]
                permissions: 0o644,
                hash: None,
                is_dir: false,
            }),
            None,
        ),
        FileChange::new(
            PathBuf::from("modified.txt"),
            ChangeType::Modified,
            Some(FileMetadata {
                path: PathBuf::from("modified.txt"),
                size: 200,
                modified: std::time::SystemTime::now(),
                #[cfg(unix)]
                permissions: 0o644,
                hash: None,
                is_dir: false,
            }),
            None,
        ),
    ];

    let summary = DiffEngine::summarize_changes(&changes);

    assert_eq!(summary.added, 1);
    assert_eq!(summary.modified, 1);
    assert_eq!(summary.files_to_sync(), 2);
    assert_eq!(summary.bytes_to_sync(), 300);
    assert!(summary.has_changes());
}
