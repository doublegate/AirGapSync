import SwiftUI
import Foundation
import Combine

/// Manages sync operations and device detection
class SyncManager: ObservableObject {
    @Published var devices: [Device] = []
    @Published var issyncing: Bool = false
    @Published var progress: Double = 0.0
    @Published var currentOperation: String = ""
    @Published var statusText: String = "Ready"
    @Published var lastSyncTime: String = "Never"
    
    var statusColor: Color {
        if issyncing {
            return .blue
        } else if devices.isEmpty {
            return .gray
        } else {
            return .green
        }
    }
    
    private var timer: Timer?
    
    init() {
        refreshDevices()
        startDeviceMonitoring()
    }
    
    deinit {
        timer?.invalidate()
    }
    
    /// Refresh the list of connected devices
    func refreshDevices() {
        // Mock device discovery for Phase 1
        // In Phase 2, this will use actual device detection
        DispatchQueue.main.async {
            self.devices = [
                Device(id: "USB001", name: "Secure Backup USB", mountPoint: "/Volumes/USB001", status: .connected),
                Device(id: "SSD001", name: "External SSD", mountPoint: "/Volumes/ExternalSSD", status: .connected)
            ]
            
            if !self.devices.isEmpty && self.statusText == "Ready" {
                self.statusText = "\(self.devices.count) device(s) connected"
            }
        }
    }
    
    /// Start monitoring for device changes
    private func startDeviceMonitoring() {
        timer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { _ in
            self.refreshDevices()
        }
    }
    
    /// Perform sync operation
    func performSync() {
        guard !issyncing else { return }
        
        issyncing = true
        progress = 0.0
        currentOperation = "Preparing sync..."
        statusText = "Syncing"
        
        // Simulate sync process
        let operations = [
            "Scanning source directory...",
            "Detecting changes...",
            "Encrypting files...",
            "Transferring data...",
            "Verifying integrity...",
            "Finalizing sync..."
        ]
        
        var currentStep = 0
        let stepProgress = 1.0 / Double(operations.count)
        
        Timer.scheduledTimer(withTimeInterval: 1.5, repeats: true) { timer in
            DispatchQueue.main.async {
                if currentStep < operations.count {
                    self.currentOperation = operations[currentStep]
                    self.progress = Double(currentStep + 1) * stepProgress
                    currentStep += 1
                } else {
                    timer.invalidate()
                    self.completSync()
                }
            }
        }
    }
    
    /// Complete the sync operation
    private func completSync() {
        issyncing = false
        progress = 1.0
        currentOperation = "Sync completed"
        statusText = "\(devices.count) device(s) connected"
        
        let formatter = DateFormatter()
        formatter.timeStyle = .short
        formatter.dateStyle = .none
        lastSyncTime = formatter.string(from: Date())
        
        // Reset progress after a delay
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
            self.progress = 0.0
            self.currentOperation = ""
        }
    }
}