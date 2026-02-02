//
//  MenuBarApp.swift
//  AirGapSync
//
//  Menu bar application for AirGapSync
//

import SwiftUI
import AppKit
import Combine
import UniformTypeIdentifiers

// MARK: - Main App

@main
struct AirGapSyncApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    
    var body: some Scene {
        Settings {
            EmptyView()
        }
    }
}

// MARK: - App Delegate

class AppDelegate: NSObject, NSApplicationDelegate {
    var statusItem: NSStatusItem!
    var popover: NSPopover!
    var eventMonitor: EventMonitor?
    @ObservedObject var syncManager = SyncManager.shared
    
    func applicationDidFinishLaunching(_ notification: Notification) {
        // Create the status item
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        
        if let button = statusItem.button {
            button.image = NSImage(systemSymbolName: "externaldrive.badge.checkmark", accessibilityDescription: "AirGapSync")
            button.action = #selector(togglePopover(_:))
            button.sendAction(on: [.leftMouseUp, .rightMouseUp])
        }
        
        // Create the popover
        popover = NSPopover()
        popover.contentSize = NSSize(width: 400, height: 500)
        popover.behavior = .transient
        popover.contentViewController = NSHostingController(rootView: ContentView())
        
        // Create event monitor
        eventMonitor = EventMonitor(mask: [.leftMouseDown, .rightMouseDown]) { [weak self] event in
            if let strongSelf = self, strongSelf.popover.isShown {
                strongSelf.closePopover(sender: nil)
            }
        }
        
        // Start monitoring for devices
        syncManager.startMonitoring()
        
        // Update status icon based on sync state
        syncManager.$syncState
            .receive(on: DispatchQueue.main)
            .sink { [weak self] state in
                self?.updateStatusIcon(for: state)
            }
            .store(in: &syncManager.cancellables)
    }
    
    @objc func togglePopover(_ sender: Any?) {
        if let event = NSApp.currentEvent, event.type == .rightMouseUp {
            showMenu()
        } else {
            if popover.isShown {
                closePopover(sender: sender)
            } else {
                showPopover(sender: sender)
            }
        }
    }
    
    func showPopover(sender: Any?) {
        if let button = statusItem.button {
            popover.show(relativeTo: button.bounds, of: button, preferredEdge: NSRectEdge.minY)
            eventMonitor?.start()
        }
    }
    
    func closePopover(sender: Any?) {
        popover.performClose(sender)
        eventMonitor?.stop()
    }
    
    func showMenu() {
        let menu = NSMenu()
        
        menu.addItem(NSMenuItem(title: "Open AirGapSync", action: #selector(showPopover(_:)), keyEquivalent: ""))
        menu.addItem(NSMenuItem.separator())
        
        if syncManager.connectedDevices.isEmpty {
            let noDevicesItem = NSMenuItem(title: "No devices connected", action: nil, keyEquivalent: "")
            noDevicesItem.isEnabled = false
            menu.addItem(noDevicesItem)
        } else {
            for device in syncManager.connectedDevices {
                let deviceItem = NSMenuItem(title: device.name, action: #selector(syncDevice(_:)), keyEquivalent: "")
                deviceItem.representedObject = device
                deviceItem.image = NSImage(systemSymbolName: device.isEncrypted ? "lock.fill" : "lock.open.fill", accessibilityDescription: nil)
                menu.addItem(deviceItem)
            }
        }
        
        menu.addItem(NSMenuItem.separator())
        menu.addItem(NSMenuItem(title: "Preferences...", action: #selector(showPreferences), keyEquivalent: ","))
        menu.addItem(NSMenuItem.separator())
        menu.addItem(NSMenuItem(title: "Quit AirGapSync", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"))
        
        statusItem.menu = menu
        statusItem.button?.performClick(nil)
        statusItem.menu = nil
    }
    
    @objc func syncDevice(_ sender: NSMenuItem) {
        guard let device = sender.representedObject as? Device else { return }
        syncManager.syncToDevice(device)
    }
    
    @objc func showPreferences() {
        // Show preferences window
        let preferencesWindow = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 600, height: 400),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        preferencesWindow.center()
        preferencesWindow.title = "AirGapSync Preferences"
        preferencesWindow.contentView = NSHostingView(rootView: PreferencesView())
        preferencesWindow.makeKeyAndOrderFront(nil)
    }
    
    func updateStatusIcon(for state: SyncState) {
        DispatchQueue.main.async { [weak self] in
            guard let button = self?.statusItem.button else { return }
            
            switch state {
            case .idle:
                button.image = NSImage(systemSymbolName: "externaldrive.badge.checkmark", accessibilityDescription: "AirGapSync")
            case .syncing:
                button.image = NSImage(systemSymbolName: "arrow.triangle.2.circlepath", accessibilityDescription: "Syncing")
            case .error:
                button.image = NSImage(systemSymbolName: "externaldrive.badge.xmark", accessibilityDescription: "Error")
            }
        }
    }
}

// MARK: - Event Monitor

class EventMonitor {
    private var monitor: Any?
    private let mask: NSEvent.EventTypeMask
    private let handler: (NSEvent?) -> Void
    
    init(mask: NSEvent.EventTypeMask, handler: @escaping (NSEvent?) -> Void) {
        self.mask = mask
        self.handler = handler
    }
    
    deinit {
        stop()
    }
    
    func start() {
        monitor = NSEvent.addGlobalMonitorForEvents(matching: mask, handler: handler)
    }
    
    func stop() {
        if monitor != nil {
            NSEvent.removeMonitor(monitor!)
            monitor = nil
        }
    }
}

// MARK: - Main Content View

struct ContentView: View {
    @StateObject private var syncManager = SyncManager.shared
    @State private var selectedDevice: Device?
    @State private var showingSettings = false
    
    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "externaldrive.connected.to.line.below")
                    .font(.title2)
                    .foregroundColor(.accentColor)
                
                Text("AirGapSync")
                    .font(.title2)
                    .fontWeight(.semibold)
                
                Spacer()
                
                Button(action: { showingSettings.toggle() }) {
                    Image(systemName: "gear")
                        .font(.title3)
                }
                .buttonStyle(PlainButtonStyle())
                .help("Settings")
            }
            .padding()
            
            Divider()
            
            // Device List
            if syncManager.connectedDevices.isEmpty {
                VStack(spacing: 16) {
                    Image(systemName: "externaldrive.badge.questionmark")
                        .font(.system(size: 48))
                        .foregroundColor(.secondary)
                    
                    Text("No devices connected")
                        .font(.headline)
                        .foregroundColor(.secondary)
                    
                    Text("Connect a removable device to start syncing")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .padding()
            } else {
                ScrollView {
                    VStack(spacing: 12) {
                        ForEach(syncManager.connectedDevices) { device in
                            DeviceRow(device: device, selectedDevice: $selectedDevice)
                        }
                    }
                    .padding()
                }
            }
            
            Divider()
            
            // Status Bar
            HStack {
                if let lastSync = syncManager.lastSyncTime {
                    Label("Last sync: \(lastSync, formatter: relativeDateFormatter)", systemImage: "clock")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Label("Never synced", systemImage: "clock")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                if syncManager.syncState == .syncing {
                    ProgressView()
                        .scaleEffect(0.7)
                }
            }
            .padding(.horizontal)
            .padding(.vertical, 8)
            .background(Color(NSColor.controlBackgroundColor))
        }
        .frame(width: 400, height: 500)
        .sheet(item: $selectedDevice) { device in
            DeviceDetailView(device: device)
        }
        .sheet(isPresented: $showingSettings) {
            SettingsView()
        }
    }
}

// MARK: - Device Row

struct DeviceRow: View {
    let device: Device
    @Binding var selectedDevice: Device?
    @StateObject private var syncManager = SyncManager.shared
    @State private var isHovering = false
    
    var body: some View {
        HStack(spacing: 12) {
            // Device Icon
            ZStack {
                Circle()
                    .fill(device.isEncrypted ? Color.green.opacity(0.1) : Color.orange.opacity(0.1))
                    .frame(width: 40, height: 40)
                
                Image(systemName: device.isEncrypted ? "lock.fill" : "lock.open.fill")
                    .foregroundColor(device.isEncrypted ? .green : .orange)
            }
            
            // Device Info
            VStack(alignment: .leading, spacing: 4) {
                Text(device.name)
                    .font(.headline)
                
                HStack(spacing: 8) {
                    Label(device.formattedCapacity, systemImage: "externaldrive")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    if let lastSync = device.lastSync {
                        Label(lastSync, formatter: relativeDateFormatter)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            
            Spacer()
            
            // Actions
            HStack(spacing: 8) {
                if syncManager.syncingDevices.contains(device.id) {
                    ProgressView()
                        .scaleEffect(0.8)
                } else {
                    Button(action: { syncManager.syncToDevice(device) }) {
                        Label("Sync", systemImage: "arrow.triangle.2.circlepath")
                            .labelStyle(.iconOnly)
                    }
                    .buttonStyle(PlainButtonStyle())
                    .help("Sync to device")
                }
                
                Button(action: { selectedDevice = device }) {
                    Image(systemName: "info.circle")
                }
                .buttonStyle(PlainButtonStyle())
                .help("Device details")
            }
        }
        .padding(12)
        .background(isHovering ? Color(NSColor.controlBackgroundColor) : Color.clear)
        .cornerRadius(8)
        .onHover { hovering in
            isHovering = hovering
        }
    }
}

// MARK: - Device Detail View

struct DeviceDetailView: View {
    let device: Device
    @Environment(\.dismiss) private var dismiss
    @StateObject private var syncManager = SyncManager.shared
    @State private var showingDeleteConfirmation = false
    
    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading) {
                    Text(device.name)
                        .font(.title2)
                        .fontWeight(.semibold)
                    
                    Text(device.id)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                Button("Done") {
                    dismiss()
                }
            }
            .padding()
            
            Divider()
            
            Form {
                // Device Info
                Section("Device Information") {
                    LabeledContent("Mount Point", value: device.mountPoint)
                    LabeledContent("Capacity", value: device.formattedCapacity)
                    LabeledContent("Available", value: device.formattedAvailable)
                    LabeledContent("Encryption", value: device.isEncrypted ? "Enabled" : "Disabled")
                }
                
                // Sync Stats
                Section("Sync Statistics") {
                    if let lastSync = device.lastSync {
                        LabeledContent("Last Sync", value: lastSync, formatter: DateFormatter.medium)
                    } else {
                        LabeledContent("Last Sync", value: "Never")
                    }
                    LabeledContent("Files Synced", value: "\(device.syncedFiles)")
                    LabeledContent("Total Size", value: device.formattedSyncSize)
                }
                
                // Actions
                Section("Actions") {
                    Button("Sync Now") {
                        syncManager.syncToDevice(device)
                        dismiss()
                    }
                    .disabled(syncManager.syncingDevices.contains(device.id))
                    
                    Button("View Sync History") {
                        // TODO: Show sync history
                    }
                    
                    Button("Forget Device", role: .destructive) {
                        showingDeleteConfirmation = true
                    }
                }
            }
            .formStyle(.grouped)
            .frame(width: 450, height: 400)
        }
        .alert("Forget Device?", isPresented: $showingDeleteConfirmation) {
            Button("Cancel", role: .cancel) { }
            Button("Forget", role: .destructive) {
                syncManager.forgetDevice(device)
                dismiss()
            }
        } message: {
            Text("This will remove the device from AirGapSync. You'll need to set it up again to sync.")
        }
    }
}

// MARK: - Settings View

struct SettingsView: View {
    @Environment(\.dismiss) private var dismiss
    @AppStorage("autoSync") private var autoSync = false
    @AppStorage("showNotifications") private var showNotifications = true
    @AppStorage("compressionLevel") private var compressionLevel = 6
    @AppStorage("verifyAfterWrite") private var verifyAfterWrite = true
    
    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Settings")
                    .font(.title2)
                    .fontWeight(.semibold)
                
                Spacer()
                
                Button("Done") {
                    dismiss()
                }
            }
            .padding()
            
            Divider()
            
            Form {
                Section("Sync Options") {
                    Toggle("Auto-sync when device connected", isOn: $autoSync)
                    Toggle("Show notifications", isOn: $showNotifications)
                    Toggle("Verify files after writing", isOn: $verifyAfterWrite)
                    
                    Picker("Compression Level", selection: $compressionLevel) {
                        Text("No compression").tag(0)
                        Text("Fast").tag(3)
                        Text("Balanced").tag(6)
                        Text("Maximum").tag(9)
                    }
                }
                
                Section("Source Directory") {
                    HStack {
                        Text(SyncManager.shared.sourceDirectory ?? "Not configured")
                            .lineLimit(1)
                            .truncationMode(.middle)
                        
                        Spacer()
                        
                        Button("Choose...") {
                            chooseSourceDirectory()
                        }
                    }
                }
                
                Section("About") {
                    LabeledContent("Version", value: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "Unknown")
                    
                    Link("View Documentation", destination: URL(string: "https://github.com/yourusername/airgapsync")!)
                    
                    Link("Report Issue", destination: URL(string: "https://github.com/yourusername/airgapsync/issues")!)
                }
            }
            .formStyle(.grouped)
            .frame(width: 500, height: 400)
        }
    }
    
    func chooseSourceDirectory() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Select Source Directory"
        
        if panel.runModal() == .OK {
            SyncManager.shared.sourceDirectory = panel.url?.path
        }
    }
}

// MARK: - Preferences View

struct PreferencesView: View {
    @State private var selectedTab = 0
    
    var body: some View {
        TabView(selection: $selectedTab) {
            GeneralPreferencesView()
                .tabItem {
                    Label("General", systemImage: "gear")
                }
                .tag(0)
            
            DevicesPreferencesView()
                .tabItem {
                    Label("Devices", systemImage: "externaldrive")
                }
                .tag(1)
            
            SecurityPreferencesView()
                .tabItem {
                    Label("Security", systemImage: "lock")
                }
                .tag(2)
            
            AdvancedPreferencesView()
                .tabItem {
                    Label("Advanced", systemImage: "gearshape.2")
                }
                .tag(3)
        }
        .frame(width: 600, height: 400)
    }
}

struct GeneralPreferencesView: View {
    @AppStorage("launchAtLogin") private var launchAtLogin = true
    @AppStorage("checkForUpdates") private var checkForUpdates = true
    
    var body: some View {
        Form {
            Toggle("Launch at login", isOn: $launchAtLogin)
            Toggle("Check for updates automatically", isOn: $checkForUpdates)
        }
        .padding()
    }
}

struct DevicesPreferencesView: View {
    var body: some View {
        Text("Device management coming soon")
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct SecurityPreferencesView: View {
    @AppStorage("requireAuthentication") private var requireAuthentication = true
    @AppStorage("encryptionAlgorithm") private var encryptionAlgorithm = "aes-256-gcm"
    
    var body: some View {
        Form {
            Toggle("Require authentication for sync", isOn: $requireAuthentication)
            
            Picker("Encryption Algorithm", selection: $encryptionAlgorithm) {
                Text("AES-256-GCM").tag("aes-256-gcm")
                Text("ChaCha20-Poly1305").tag("chacha20-poly1305")
            }
        }
        .padding()
    }
}

struct AdvancedPreferencesView: View {
    @AppStorage("chunkSizeMB") private var chunkSizeMB = 1
    @AppStorage("parallelWorkers") private var parallelWorkers = 4
    
    var body: some View {
        Form {
            Stepper("Chunk Size: \(chunkSizeMB) MB", value: $chunkSizeMB, in: 1...16)
            Stepper("Parallel Workers: \(parallelWorkers)", value: $parallelWorkers, in: 1...8)
        }
        .padding()
    }
}

// MARK: - Helpers

let relativeDateFormatter: RelativeDateTimeFormatter = {
    let formatter = RelativeDateTimeFormatter()
    formatter.unitsStyle = .abbreviated
    return formatter
}()

extension DateFormatter {
    static let medium: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter
    }()
}