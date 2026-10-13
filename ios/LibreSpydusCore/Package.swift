// swift-tools-version: 6.0
// LibreSpydusCore: the Rust core (core/) for the iOS app.
//   - binary target: LibreSpydusCore.xcframework, built by ios/scripts/build-core.sh (gitignored)
//   - Swift target: the UniFFI-generated bindings. Sources/LibreSpydusCore/librespydus_core.swift
//     is a symlink to core/generated/swift/librespydus_core.swift (single source of truth).
import PackageDescription

let package = Package(
    name: "LibreSpydusCore",
    platforms: [.iOS(.v17)],
    products: [
        .library(name: "LibreSpydusCore", targets: ["LibreSpydusCore"]),
    ],
    targets: [
        .binaryTarget(name: "librespydus_coreFFI", path: "LibreSpydusCore.xcframework"),
        .target(
            name: "LibreSpydusCore",
            dependencies: ["librespydus_coreFFI"],
            path: "Sources/LibreSpydusCore"
        ),
    ],
    swiftLanguageModes: [.v6]
)
