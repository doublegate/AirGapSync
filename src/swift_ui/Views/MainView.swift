import SwiftUI

struct MainView: View {
    @StateObject private var syncManager = SyncManager()
    @State private var selectedTab = 0
    
    var body: some View {
        VStack(spacing: 0) {
            // Header
            HeaderView(syncManager: syncManager)
            
            Divider()
            
            // Tab selection
            Picker("", selection: $selectedTab) {
                Text("Status").tag(0)
                Text("Devices").tag(1)
                Text("Settings").tag(2)
            }
            .pickerStyle(SegmentedPickerStyle())
            .padding(.horizontal)
            .padding(.top, 8)
            
            // Content
            TabView(selection: $selectedTab) {
                StatusView(syncManager: syncManager)
                    .tag(0)
                
                DevicesView(syncManager: syncManager)
                    .tag(1)
                
                SettingsView()
                    .tag(2)
            }
            .tabViewStyle(PageTabViewStyle(indexDisplayMode: .never))
            .frame(height: 300)
            
            Divider()
            
            // Footer
            FooterView()
        }
        .frame(width: 300, height: 400)
        .background(Color(NSColor.controlBackgroundColor))
    }
}

#Preview {
    MainView()
}