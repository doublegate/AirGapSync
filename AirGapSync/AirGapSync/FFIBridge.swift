//
//  FFIBridge.swift
//  AirGapSync
//
//  Swift wrapper around the Rust FFI for type-safe access
//

import Foundation

/// Swift-friendly wrapper around FFI error codes
enum AirGapError: Error, CustomStringConvertible {
    case nullPointer
    case invalidUtf8
    case configError(String)
    case cryptoError(String)
    case keychainError(String)
    case ioError(String)
    case deviceNotFound(String)
    case syncError(String)
    case unknown(String)

    init(from result: FFIResult) {
        let message: String? = result.error_message.flatMap { ptr in
            guard ptr != nil else { return nil }
            defer { airgap_free_string(UnsafeMutablePointer(mutating: ptr)) }
            return String(cString: ptr!)
        }

        switch result.error_code {
        case Success:
            self = .unknown("Success should not create error")
        case NullPointer:
            self = .nullPointer
        case InvalidUtf8:
            self = .invalidUtf8
        case ConfigError:
            self = .configError(message ?? "Unknown config error")
        case CryptoError:
            self = .cryptoError(message ?? "Unknown crypto error")
        case KeychainError:
            self = .keychainError(message ?? "Unknown keychain error")
        case IoError:
            self = .ioError(message ?? "Unknown IO error")
        case DeviceNotFound:
            self = .deviceNotFound(message ?? "Device not found")
        case SyncError:
            self = .syncError(message ?? "Unknown sync error")
        case Unknown:
            self = .unknown(message ?? "Unknown error")
        default:
            self = .unknown(message ?? "Unrecognized error code")
        }
    }

    var description: String {
        switch self {
        case .nullPointer: return "Null pointer error"
        case .invalidUtf8: return "Invalid UTF-8 string"
        case .configError(let msg): return "Configuration error: \(msg)"
        case .cryptoError(let msg): return "Cryptography error: \(msg)"
        case .keychainError(let msg): return "Keychain error: \(msg)"
        case .ioError(let msg): return "IO error: \(msg)"
        case .deviceNotFound(let msg): return "Device not found: \(msg)"
        case .syncError(let msg): return "Sync error: \(msg)"
        case .unknown(let msg): return "Unknown error: \(msg)"
        }
    }
}

/// Swift-friendly sync options
struct AirGapSyncOptions {
    var dryRun: Bool = false
    var verbose: Bool = false
    var parallelWorkers: Int = 4
    var chunkSize: Int = 1024 * 1024 // 1MB
    var verifyAfterWrite: Bool = true
    var resume: Bool = true
    var compressionLevel: Int = 6
    var showProgress: Bool = true
    var maxRetries: Int = 3

    func toFFI() -> FFISyncOptions {
        return FFISyncOptions(
            dry_run: dryRun,
            verbose: verbose,
            parallel_workers: UInt32(parallelWorkers),
            chunk_size: UInt64(chunkSize),
            verify_after_write: verifyAfterWrite,
            resume: resume,
            compression_level: UInt32(compressionLevel),
            show_progress: showProgress,
            max_retries: UInt32(maxRetries)
        )
    }
}

/// Swift-friendly sync result
struct AirGapSyncResult {
    let filesSynced: Int
    let bytesTransferred: Int
    let filesSkipped: Int
    let filesFailed: Int
    let durationSeconds: Int
    let averageTransferRate: Int
    let snapshotId: String?

    init(from ffiResult: FFISyncResult) {
        self.filesSynced = Int(ffiResult.files_synced)
        self.bytesTransferred = Int(ffiResult.bytes_transferred)
        self.filesSkipped = Int(ffiResult.files_skipped)
        self.filesFailed = Int(ffiResult.files_failed)
        self.durationSeconds = Int(ffiResult.duration_seconds)
        self.averageTransferRate = Int(ffiResult.average_transfer_rate)

        if let ptr = ffiResult.snapshot_id {
            self.snapshotId = String(cString: ptr)
        } else {
            self.snapshotId = nil
        }
    }
}

/// Swift wrapper for the Rust sync engine
class AirGapSyncEngine {
    private var engine: OpaquePointer?
    private var config: OpaquePointer?

    init(configPath: String) throws {
        // Initialize the library
        let initResult = airgap_initialize()
        if initResult.error_code != Success {
            throw AirGapError(from: initResult)
        }

        // Load configuration
        config = configPath.withCString { path in
            airgap_load_config(path)
        }

        guard config != nil else {
            throw AirGapError.configError("Failed to load config from: \(configPath)")
        }

        // Create sync engine
        engine = airgap_create_engine(config)
        guard engine != nil else {
            airgap_free_config(config)
            config = nil
            throw AirGapError.syncError("Failed to create sync engine")
        }
    }

    deinit {
        if let engine = engine {
            airgap_free_engine(engine)
        }
        if let config = config {
            airgap_free_config(config)
        }
    }

    func sync(deviceId: String, options: AirGapSyncOptions = AirGapSyncOptions()) throws -> AirGapSyncResult {
        guard let engine = engine else {
            throw AirGapError.syncError("Sync engine not initialized")
        }

        var ffiOptions = options.toFFI()
        var ffiResult = FFISyncResult(
            files_synced: 0,
            bytes_transferred: 0,
            files_skipped: 0,
            files_failed: 0,
            duration_seconds: 0,
            average_transfer_rate: 0,
            snapshot_id: nil
        )

        let result = deviceId.withCString { deviceIdPtr in
            airgap_sync(engine, deviceIdPtr, &ffiOptions, nil, nil, &ffiResult)
        }

        if result.error_code != Success {
            throw AirGapError(from: result)
        }

        return AirGapSyncResult(from: ffiResult)
    }

    func getDeviceInfo(deviceId: String) throws -> (id: String, name: String, mountPoint: String, isEncrypted: Bool, snapshotCount: Int) {
        guard let config = config else {
            throw AirGapError.configError("Config not loaded")
        }

        let deviceInfo = deviceId.withCString { deviceIdPtr in
            airgap_get_device_info(config, deviceIdPtr)
        }

        guard let info = deviceInfo else {
            throw AirGapError.deviceNotFound(deviceId)
        }

        defer { airgap_free_device_info(info) }

        let id = String(cString: info.pointee.id)
        let name = String(cString: info.pointee.name)
        let mountPoint = String(cString: info.pointee.mount_point)
        let isEncrypted = info.pointee.is_encrypted
        let snapshotCount = Int(info.pointee.snapshot_count)

        return (id, name, mountPoint, isEncrypted, snapshotCount)
    }

    func listSnapshots(deviceId: String) throws -> [String] {
        guard let config = config else {
            throw AirGapError.configError("Config not loaded")
        }

        var count: UInt32 = 0
        let snapshotsPtr = deviceId.withCString { deviceIdPtr in
            airgap_list_snapshots(config, deviceIdPtr, &count)
        }

        guard snapshotsPtr != nil, count > 0 else {
            return []
        }

        defer { airgap_free_snapshot_list(snapshotsPtr, count) }

        var snapshots: [String] = []
        let buffer = UnsafeBufferPointer(start: snapshotsPtr, count: Int(count))
        for ptr in buffer {
            if let snapshotPtr = ptr {
                snapshots.append(String(cString: snapshotPtr))
            }
        }

        return snapshots
    }

    func verifySnapshot(deviceId: String, snapshotId: String) throws {
        guard let config = config else {
            throw AirGapError.configError("Config not loaded")
        }

        let result = deviceId.withCString { deviceIdPtr in
            snapshotId.withCString { snapshotIdPtr in
                airgap_verify_snapshot(config, deviceIdPtr, snapshotIdPtr)
            }
        }

        if result.error_code != Success {
            throw AirGapError(from: result)
        }
    }

    static func getVersion() -> String {
        guard let ptr = airgap_get_version() else {
            return "Unknown"
        }
        defer { airgap_free_string(UnsafeMutablePointer(mutating: ptr)) }
        return String(cString: ptr)
    }
}
