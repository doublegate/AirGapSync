import SwiftUI

struct HeaderView: View {
    @ObservedObject var syncManager: SyncManager
    
    var body: some View {
        VStack(spacing: 4) {
            HStack {
                Image(systemName: "externaldrive.fill")
                    .font(.title2)
                    .foregroundColor(.accentColor)
                
                VStack(alignment: .leading) {
                    Text("AirGapSync")
                        .font(.headline)
                        .fontWeight(.semibold)
                    
                    Text(syncManager.statusText)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                statusIndicator
            }
            .padding(.horizontal)
            .padding(.top, 12)
            .padding(.bottom, 8)
        }
        .background(Color(NSColor.controlBackgroundColor))
    }
    
    private var statusIndicator: some View {
        Circle()
            .fill(syncManager.statusColor)
            .frame(width: 8, height: 8)
            .overlay(
                Circle()
                    .stroke(Color.primary.opacity(0.2), lineWidth: 1)
            )
    }
}

#Preview {
    HeaderView(syncManager: SyncManager())
}