import SwiftUI

struct DevicesView: View {
    @ObservedObject var syncManager: SyncManager
    
    var body: some View {
        VStack(spacing: 12) {
            // Device list header
            HStack {
                Text("Connected Devices")
                    .font(.headline)
                
                Spacer()
                
                Button(action: {
                    syncManager.refreshDevices()
                }) {
                    Image(systemName: "arrow.clockwise")
                        .foregroundColor(.accentColor)
                }
                .buttonStyle(PlainButtonStyle())
            }
            .padding(.horizontal)
            
            // Device list
            if syncManager.devices.isEmpty {
                VStack(spacing: 8) {
                    Image(systemName: "externaldrive.slash")
                        .font(.system(size: 32))
                        .foregroundColor(.secondary)
                    
                    Text("No devices connected")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                    
                    Text("Connect a removable device to begin syncing")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ScrollView {
                    LazyVStack(spacing: 8) {
                        ForEach(syncManager.devices, id: \.id) { device in
                            DeviceRowView(device: device)
                        }
                    }
                    .padding(.horizontal)
                }
            }
            
            Spacer()
        }
        .padding(.vertical)
    }
}

struct DeviceRowView: View {
    let device: Device
    
    var body: some View {
        HStack(spacing: 12) {
            // Device icon
            Image(systemName: device.iconName)
                .font(.title2)
                .foregroundColor(.accentColor)
                .frame(width: 24)
            
            // Device info
            VStack(alignment: .leading, spacing: 2) {
                Text(device.name)
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                Text(device.mountPoint)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            // Status and capacity
            VStack(alignment: .trailing, spacing: 2) {
                HStack(spacing: 4) {
                    Circle()
                        .fill(device.statusColor)
                        .frame(width: 6, height: 6)
                    Text(device.status)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                if let capacity = device.capacity {
                    Text(capacity)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(8)
        .background(Color(NSColor.controlBackgroundColor))
        .cornerRadius(6)
    }
}

#Preview {
    DevicesView(syncManager: SyncManager())
}