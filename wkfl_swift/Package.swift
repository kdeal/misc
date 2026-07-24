// swift-tools-version: 6.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
  name: "wkfl",
  products: [
    .executable(name: "wkfl", targets: ["WkflCli"]),
    .library(name: "WkflLib", targets: ["WkflLib"]),
  ],
  dependencies: [
    .package(url: "https://github.com/apple/swift-argument-parser.git", from: "1.7.0")
  ],
  targets: [
    .target(name: "WkflLib"),
    .executableTarget(
      name: "WkflCli",
      dependencies: [
        .product(name: "ArgumentParser", package: "swift-argument-parser"),
        .target(name: "WkflLib"),
      ]
    ),
    .testTarget(
      name: "WkflLibTests",
      dependencies: ["WkflLib"]
    ),
    .testTarget(
      name: "WkflCliTests",
      dependencies: ["WkflCli"]
    ),
  ],
  swiftLanguageModes: [.v6]
)
