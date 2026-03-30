// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "lifelog-compression",
    platforms: [
        .macOS(.v14),
    ],
    products: [
        .executable(
            name: "lifelog-compression",
            targets: ["LifelogCompressionCLI"]
        ),
    ],
    targets: [
        .executableTarget(
            name: "LifelogCompressionCLI"
        ),
    ]
)
