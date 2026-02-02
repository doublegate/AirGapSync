//
//  SyncManager.swift
//  AirGapSync
//
//  Manages sync operations and device monitoring
//

import Foundation
import Combine
import AppKit
import DiskArbitration

// MARK: - Models

struct Device: Identifiable, Equatable {
    let id: String
    let name: String
    let mountPoint: String
    let capacity: Int64
    let available: Int64
    let isEncrypted: Bool
    let lastSync: Date?
    let syncedFiles: Int
    let syncedBytes: Int64
    
    var formattedCapacity: String {
        ByteCountFormatter.string(fromByteCount: capacity, countStyle: .file)
    }
    
    var formattedAvailable: String {
        ByteCountFormatter.string(fromByteCount: available, countStyle: .file)
    }
    
    var formattedSyncSize: String {
        ByteCountFormatter.string(fromByteCount: syncedBytes, countStyle: .file)
    }
}

enum SyncState {
    case idle
    case syncing
    case error(String)
}

// MARK: - Sync Manager

class SyncManager: ObservableObject {
    static let shared = SyncManager()
    
    @Published var connectedDevices: [Device] = []
    @Published var syncState: SyncState = .idle
    @Published var syncingDevices: Set<String> = []
    @Published var lastSyncTime: Date?
    @Published var sourceDirectory: String? {
        didSet {
            UserDefaults.standard.set(sourceDirectory, forKey: "sourceDirectory")
        }
    }
    
    var cancellables = Set<AnyCancellable>()
    private var diskMonitor: DiskMonitor?
    private let syncQueue = DispatchQueue(label: "com.airgapsync.sync", qos: .userInitiated)
    
    private init() {
        sourceDirectory = UserDefaults.standard.string(forKey: "sourceDirectory")
        diskMonitor = DiskMonitor()
        
        // Subscribe to disk events
        diskMonitor?.deviceMounted
            .receive(on: DispatchQueue.main)
            .sink { [weak self] device in
                self?.handleDeviceMounted(device)
            }
            .store(in: &cancellables)
        
        diskMonitor?.deviceUnmounted
            .receive(on: DispatchQueue.main)
            .sink { [weak self] deviceID in
                self?.handleDeviceUnmounted(deviceID)
            }
            .store(in: &cancellables)
    }
    
    func startMonitoring() {
        diskMonitor?.start()
        refreshDevices()
    }
    
    func stopMonitoring() {
        diskMonitor?.stop()
    }
    
    func refreshDevices() {
        // Get mounted volumes
        let fileManager = FileManager.default
        guard let volumes = fileManager.mountedVolumeURLs(includingResourceValuesForKeys: [
            .volumeNameKey,
            .volumeIdentifierKey,
            .volumeTotalCapacityKey,
            .volumeAvailableCapacityKey,
            .volumeIsRemovableKey,
            .volumeIsEjectableKey
        ], options: [.skipHiddenVolumes]) else { return }
        
        var devices: [Device] = []
        
        for volume in volumes {
            do {
                let resourceValues = try volume.resourceValues(forKeys: [
                    .volumeNameKey,
                    .volumeIdentifierKey,
                    .volumeTotalCapacityKey,
                    .volumeAvailableCapacityKey,
                    .volumeIsRemovableKey,
                    .volumeIsEjectableKey
                ])
                
                // Only include removable/ejectable volumes
                if let isRemovable = resourceValues.volumeIsRemovable,
                   let isEjectable = resourceValues.volumeIsEjectable,
                   (isRemovable || isEjectable) {
                    
                    let device = Device(
                        id: resourceValues.volumeIdentifier ?? UUID().uuidString,
                        name: resourceValues.volumeName ?? "Unknown Device",
                        mountPoint: volume.path,
                        capacity: Int64(resourceValues.volumeTotalCapacity ?? 0),
                        available: Int64(resourceValues.volumeAvailableCapacity ?? 0),
                        isEncrypted: checkIfEncrypted(volume),
                        lastSync: getLastSyncTime(for: resourceValues.volumeIdentifier ?? ""),
                        syncedFiles: getSyncedFiles(for: resourceValues.volumeIdentifier ?? ""),
                        syncedBytes: getSyncedBytes(for: resourceValues.volumeIdentifier ?? "")
                    )
                    
                    devices.append(device)
                }
            } catch {
                print("Error reading volume info: \(error)")
            }
        }
        
        DispatchQueue.main.async {
            self.connectedDevices = devices
        }
    }
    
    func syncToDevice(_ device: Device) {
        guard let sourceDir = sourceDirectory else {
            syncState = .error("No source directory configured")
            return
        }
        
        syncingDevices.insert(device.id)
        syncState = .syncing
        
        syncQueue.async { [weak self] in
            // Call Rust sync engine via FFI or command line
            let result = self?.performSync(from: sourceDir, to: device)
            
            DispatchQueue.main.async {
                self?.syncingDevices.remove(device.id)
                
                if let error = result?.error {
                    self?.syncState = .error(error)
                } else {
                    self?.syncState = .idle
                    self?.lastSyncTime = Date()
                    self?.saveSyncMetadata(for: device)
                    
                    // Show notification
                    if UserDefaults.standard.bool(forKey: "showNotifications") {
                        self?.showSyncNotification(for: device, success: true)
                    }
                }
                
                // Refresh device info
                self?.refreshDevices()
            }
        }
    }
    
    func forgetDevice(_ device: Device) {
        // Remove device metadata
        UserDefaults.standard.removeObject(forKey: "lastSync_\(device.id)")
        UserDefaults.standard.removeObject(forKey: "syncedFiles_\(device.id)")
        UserDefaults.standard.removeObject(forKey: "syncedBytes_\(device.id)")
        
        // Remove from connected devices
        connectedDevices.removeAll { $0.id == device.id }
    }
    
    // MARK: - Private Methods
    
    private func handleDeviceMounted(_ device: Device) {
        // Add to connected devices if not already present
        if !connectedDevices.contains(where: { $0.id == device.id }) {
            connectedDevices.append(device)
            
            // Auto-sync if enabled
            if UserDefaults.standard.bool(forKey: "autoSync") {
                syncToDevice(device)
            }
        }
    }
    
    private func handleDeviceUnmounted(_ deviceID: String) {
        connectedDevices.removeAll { $0.id == deviceID }
        syncingDevices.remove(deviceID)
    }
    
    private func checkIfEncrypted(_ volume: URL) -> Bool {
        // Check for .airgapsync directory with encryption markers
        let airgapPath = volume.appendingPathComponent(".airgapsync")
        let encryptionMarker = airgapPath.appendingPathComponent("encrypted")
        return FileManager.default.fileExists(atPath: encryptionMarker.path)
    }
    
    private func getLastSyncTime(for deviceID: String) -> Date? {
        UserDefaults.standard.object(forKey: "lastSync_\(deviceID)") as? Date
    }
    
    private func getSyncedFiles(for deviceID: String) -> Int {
        UserDefaults.standard.integer(forKey: "syncedFiles_\(deviceID)")
    }
    
    private func getSyncedBytes(for deviceID: String) -> Int64 {
        Int64(UserDefaults.standard.integer(forKey: "syncedBytes_\(deviceID)"))
    }
    
    private func saveSyncMetadata(for device: Device) {
        UserDefaults.standard.set(Date(), forKey: "lastSync_\(device.id)")
        // These would be updated from actual sync results
        UserDefaults.standard.set(device.syncedFiles, forKey: "syncedFiles_\(device.id)")
        UserDefaults.standard.set(device.syncedBytes, forKey: "syncedBytes_\(device.id)")
    }
    
    private func performSync(from source: String, to device: Device) -> (error: String?) {
        // Create temporary config file
        let configURL = FileManager.default.temporaryDirectory.appendingPathComponent("airgapsync_config_\(UUID().uuidString).toml")
        
        do {
            // Create config content
            let config = """
            [general]
            verbose = false
            
            [source]
            path = "\(source)"
            exclude = [".DS_Store", "*.tmp"]
            
            [[device]]
            id = "\(device.id)"
            name = "\(device.name)"
            mount_point = "\(device.mountPoint)"
            
            [device.encryption]
            algorithm = "\(UserDefaults.standard.string(forKey: "encryptionAlgorithm") ?? "aes-256-gcm")"
            
            [policy]
            compression_level = \(UserDefaults.standard.integer(forKey: "compressionLevel"))
            verify_after_write = \(UserDefaults.standard.bool(forKey: "verifyAfterWrite"))
            """
            
            try config.write(to: configURL, atomically: true, encoding: .utf8)
            
            // Call airgapsync CLI
            let task = Process()
            task.executableURL = URL(fileURLWithPath: "/usr/local/bin/airgapsync")
            task.arguments = [
                "sync",
                device.id,
                "--config", configURL.path,
                "--parallel", String(UserDefaults.standard.integer(forKey: "parallelWorkers")),
                "--chunk-size-mb", String(UserDefaults.standard.integer(forKey: "chunkSizeMB"))
            ]
            
            let pipe = Pipe()
            task.standardOutput = pipe
            task.standardError = pipe
            
            try task.run()
            task.waitUntilExit()
            
            // Clean up config
            try? FileManager.default.removeItem(at: configURL)
            
            if task.terminationStatus != 0 {
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                let output = String(data: data, encoding: .utf8) ?? "Unknown error"
                return (error: output)
            }
            
            return (error: nil)
            
        } catch {
            try? FileManager.default.removeItem(at: configURL)
            return (error: error.localizedDescription)
        }
    }
    
    private func showSyncNotification(for device: Device, success: Bool) {
        let notification = NSUserNotification()
        notification.title = success ? "Sync Complete" : "Sync Failed"
        notification.informativeText = success 
            ? "\(device.name) has been synced successfully"
            : "Failed to sync \(device.name)"
        notification.soundName = success ? NSUserNotificationDefaultSoundName : nil
        
        NSUserNotificationCenter.default.deliver(notification)
    }
}

// MARK: - Disk Monitor

class DiskMonitor {
    let deviceMounted = PassthroughSubject<Device, Never>()
    let deviceUnmounted = PassthroughSubject<String, Never>()
    
    private var session: DASession?
    private let queue = DispatchQueue(label: "com.airgapsync.diskmonitor")
    
    init() {
        session = DASessionCreate(kCFAllocatorDefault)
        if let session = session {
            DASessionSetDispatchQueue(session, queue)
        }
    }
    
    func start() {
        guard let session = session else { return }
        
        // Register callbacks
        DARegisterDiskAppearedCallback(session, nil, diskAppearedCallback, Unmanaged.passUnretained(self).toOpaque())
        DARegisterDiskDisappearedCallback(session, nil, diskDisappearedCallback, Unmanaged.passUnretained(self).toOpaque())
    }
    
    func stop() {
        guard let session = session else { return }
        
        DAUnregisterCallback(session, Unmanaged.passUnretained(self).toOpaque())
    }
}

// MARK: - DiskArbitration Callbacks

func diskAppearedCallback(disk: DADisk, context: UnsafeMutableRawPointer?) {
    guard let monitor = Unmanaged<DiskMonitor>.fromOpaque(context!).takeUnretainedValue() as DiskMonitor? else { return }
    
    if let info = DADiskCopyDescription(disk) as? [String: Any],
       let volumePath = info[kDADiskDescriptionVolumePathKey as String] as? URL {
        
        // Create device info and notify
        let device = Device(
            id: (info[kDADiskDescriptionVolumeUUIDKey as String] as? CFUUID).map { CFUUIDCreateString(nil, $0) as String } ?? UUID().uuidString,
            name: info[kDADiskDescriptionVolumeNameKey as String] as? String ?? "Unknown",
            mountPoint: volumePath.path,
            capacity: info[kDADiskDescriptionMediaSizeKey as String] as? Int64 ?? 0,
            available: 0, // Would need to query separately
            isEncrypted: false, // Would need to check
            lastSync: nil,
            syncedFiles: 0,
            syncedBytes: 0
        )
        
        monitor.deviceMounted.send(device)
    }
}

func diskDisappearedCallback(disk: DADisk, context: UnsafeMutableRawPointer?) {
    guard let monitor = Unmanaged<DiskMonitor>.fromOpaque(context!).takeUnretainedValue() as DiskMonitor? else { return }
    
    if let info = DADiskCopyDescription(disk) as? [String: Any],
       let uuid = (info[kDADiskDescriptionVolumeUUIDKey as String] as? CFUUID).map({ CFUUIDCreateString(nil, $0) as String }) {
        monitor.deviceUnmounted.send(uuid)
    }
}