// swift-tools-version: 6.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "wkfl",
    products: [
        .library(name: "WkflLib", targets: ["WkflLib"])
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser.git", from: "1.7.0")
    ],
    targets: [
        // Targets are the basic building blocks of a package, defining a module or a test suite.
        // Targets can depend on other targets in this package and products from dependencies.
        .executableTarget(
            name: "WkflCli",
            dependencies: [
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
                .target(name: "WkflLib"),
            ]
        ),
        .testTarget(
            name: "WkflTests",
            dependencies: ["WkflCli", "WkflLib"]
        ),
        .target(name: "WkflLib"),
    ],
    swiftLanguageModes: [.v6]
)
