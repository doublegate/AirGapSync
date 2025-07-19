import SwiftUI
import Foundation

/// Represents a removable storage device
struct Device: Identifiable, Hashable {
    let id: String
    let name: String
    let mountPoint: String
    let status: String
    let capacity: String?
    let iconName: String
    let statusColor: Color
    
    init(id: String, name: String, mountPoint: String, status: DeviceStatus = .disconnected) {
        self.id = id
        self.name = name
        self.mountPoint = mountPoint
        
        switch status {
        case .connected:
            self.status = "Connected"
            self.statusColor = .green
        case .syncing:
            self.status = "Syncing"
            self.statusColor = .blue
        case .error:
            self.status = "Error"
            self.statusColor = .red
        case .disconnected:
            self.status = "Disconnected"
            self.statusColor = .gray
        }
        
        // Determine icon based on device type
        if mountPoint.contains("USB") || name.lowercased().contains("usb") {
            self.iconName = "externaldrive.fill"
        } else if name.lowercased().contains("ssd") {
            self.iconName = "internaldrive.fill"
        } else {
            self.iconName = "externaldrive"
        }
        
        // Mock capacity for demo
        self.capacity = "256 GB available"
    }
}

enum DeviceStatus {
    case connected
    case syncing
    case error
    case disconnected
}