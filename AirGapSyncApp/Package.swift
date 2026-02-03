// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "AirGapSyncApp",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .executable(
            name: "AirGapSyncApp",
            targets: ["AirGapSyncApp"])
    ],
    targets: [
        .systemLibrary(
            name: "CAirGapSync",
            path: "Sources/CAirGapSync",
            pkgConfig: "airgapsync"
        ),
        .executableTarget(
            name: "AirGapSyncApp",
            dependencies: ["CAirGapSync"],
            path: "Sources/AirGapSyncApp",
            swiftSettings: [
                .unsafeFlags(["-I", "../lib"])
            ],
            linkerSettings: [
                .unsafeFlags(["-L", "../lib", "-L", "../target/release"]),
                .linkedLibrary("airgap_sync"),
                .linkedLibrary("resolv"),
                .linkedFramework("Security"),
                .linkedFramework("DiskArbitration"),
                .linkedFramework("AppKit")
            ]
        )
    ]
)
