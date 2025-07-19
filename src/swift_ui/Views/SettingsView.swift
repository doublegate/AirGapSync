import SwiftUI

struct SettingsView: View {
    @AppStorage("syncAutomatically") private var syncAutomatically = false
    @AppStorage("enableNotifications") private var enableNotifications = true
    @AppStorage("retainSnapshots") private var retainSnapshots = 3
    @AppStorage("sourceDirectory") private var sourceDirectory = "~/Documents"
    
    var body: some View {
        VStack(spacing: 16) {
            // General Settings
            VStack(alignment: .leading, spacing: 8) {
                Text("General")
                    .font(.headline)
                    .padding(.bottom, 4)
                
                Toggle("Sync automatically", isOn: $syncAutomatically)
                    .toggleStyle(SwitchToggleStyle())
                
                Toggle("Enable notifications", isOn: $enableNotifications)
                    .toggleStyle(SwitchToggleStyle())
                
                Divider()
                
                VStack(alignment: .leading, spacing: 4) {
                    Text("Retain snapshots")
                        .font(.subheadline)
                    
                    Stepper(value: $retainSnapshots, in: 1...10) {
                        Text("\(retainSnapshots) snapshots")
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding()
            .background(Color(NSColor.controlBackgroundColor))
            .cornerRadius(8)
            
            // Source Directory
            VStack(alignment: .leading, spacing: 8) {
                Text("Source Directory")
                    .font(.headline)
                    .padding(.bottom, 4)
                
                HStack {
                    Text(sourceDirectory)
                        .font(.body)
                        .foregroundColor(.secondary)
                    
                    Spacer()
                    
                    Button("Choose...") {
                        chooseSourceDirectory()
                    }
                    .buttonStyle(BorderedButtonStyle())
                }
            }
            .padding()
            .background(Color(NSColor.controlBackgroundColor))
            .cornerRadius(8)
            
            Spacer()
            
            // Action buttons
            VStack(spacing: 8) {
                Button("Open Configuration") {
                    openConfigurationFile()
                }
                .buttonStyle(BorderedProminentButtonStyle())
                
                Button("Show Logs") {
                    showLogFile()
                }
                .buttonStyle(BorderedButtonStyle())
                
                Button("About AirGapSync") {
                    showAbout()
                }
                .buttonStyle(PlainButtonStyle())
                .foregroundColor(.secondary)
            }
        }
        .padding()
    }
    
    private func chooseSourceDirectory() {
        let openPanel = NSOpenPanel()
        openPanel.allowsMultipleSelection = false
        openPanel.canChooseDirectories = true
        openPanel.canChooseFiles = false
        openPanel.canCreateDirectories = true
        
        if openPanel.runModal() == .OK {
            if let url = openPanel.url {
                sourceDirectory = url.path
            }
        }
    }
    
    private func openConfigurationFile() {
        let configPath = NSHomeDirectory() + "/.airgapsync/config.toml"
        let url = URL(fileURLWithPath: configPath)
        NSWorkspace.shared.open(url)
    }
    
    private func showLogFile() {
        let logPath = NSHomeDirectory() + "/.airgapsync/sync.log"
        let url = URL(fileURLWithPath: logPath)
        NSWorkspace.shared.open(url)
    }
    
    private func showAbout() {
        NSApplication.shared.orderFrontStandardAboutPanel(nil)
    }
}

#Preview {
    SettingsView()
}