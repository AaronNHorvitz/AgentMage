// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "AgentMageMacOSPlatform",
    platforms: [.macOS(.v15)],
    products: [
        .library(
            name: "AgentMageMacOSPlatform",
            targets: ["AgentMageMacOSPlatform"]
        )
    ],
    targets: [
        .target(
            name: "AgentMageMacOSPlatform",
            path: "Sources/AgentMageMacOSPlatform"
        ),
        .testTarget(
            name: "AgentMageMacOSPlatformTests",
            dependencies: ["AgentMageMacOSPlatform"],
            path: "Tests/AgentMageMacOSPlatformTests"
        )
    ]
)
