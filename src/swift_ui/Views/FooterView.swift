import SwiftUI

struct FooterView: View {
    var body: some View {
        VStack(spacing: 4) {
            Divider()
            
            HStack {
                Button("Preferences...") {
                    openPreferences()
                }
                .buttonStyle(PlainButtonStyle())
                .foregroundColor(.secondary)
                .font(.caption)
                
                Spacer()
                
                Button("Quit AirGapSync") {
                    NSApplication.shared.terminate(nil)
                }
                .buttonStyle(PlainButtonStyle())
                .foregroundColor(.secondary)
                .font(.caption)
            }
            .padding(.horizontal)
            .padding(.vertical, 8)
        }
        .background(Color(NSColor.controlBackgroundColor))
    }
    
    private func openPreferences() {
        if let window = NSApplication.shared.windows.first {
            window.makeKeyAndOrderFront(nil)
            NSApplication.shared.activate(ignoringOtherApps: true)
        }
    }
}

#Preview {
    FooterView()
}