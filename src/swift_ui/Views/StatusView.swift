import SwiftUI

struct StatusView: View {
    @ObservedObject var syncManager: SyncManager
    
    var body: some View {
        VStack(spacing: 12) {
            // Sync status card
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Sync Status")
                        .font(.headline)
                    
                    Spacer()
                    
                    Button(action: {
                        syncManager.performSync()
                    }) {
                        Image(systemName: "arrow.clockwise")
                            .foregroundColor(.accentColor)
                    }
                    .buttonStyle(PlainButtonStyle())
                }
                
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Last Sync")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        Text(syncManager.lastSyncTime)
                            .font(.body)
                    }
                    
                    Spacer()
                    
                    VStack(alignment: .trailing, spacing: 4) {
                        Text("Status")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        HStack(spacing: 4) {
                            Circle()
                                .fill(syncManager.statusColor)
                                .frame(width: 6, height: 6)
                            Text(syncManager.statusText)
                                .font(.body)
                        }
                    }
                }
            }
            .padding()
            .background(Color(NSColor.controlBackgroundColor))
            .cornerRadius(8)
            
            // Progress indicator
            if syncManager.issyncing {
                VStack(spacing: 4) {
                    ProgressView(value: syncManager.progress)
                        .progressViewStyle(LinearProgressViewStyle())
                    
                    HStack {
                        Text(syncManager.currentOperation)
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        Spacer()
                        
                        Text("\(Int(syncManager.progress * 100))%")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                .padding(.horizontal)
            }
            
            Spacer()
        }
        .padding()
    }
}

#Preview {
    StatusView(syncManager: SyncManager())
}