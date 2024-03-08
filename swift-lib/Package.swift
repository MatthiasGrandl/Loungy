// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "LoungyLibrary",
    platforms: [
        .macOS(.v11),
    ],
    products: [
        .library(
            name: "LoungyLibrary",
            type: .static,
            targets: ["LoungyLibrary"]
        ),
    ],
    dependencies: [
        .package(
            name: "SwiftRs",
            url: "https://github.com/Brendonovich/swift-rs",
            from: "1.0.6"
        ),
    ],
    targets: [
        .target(
            name: "LoungyLibrary",
            dependencies: [
                .product(
                    name: "SwiftRs",
                    package: "SwiftRs"
                )
            ],
            path: "Sources"
        )
    ]
)
