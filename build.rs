use swift_rs::SwiftLinker;

fn main() {
    SwiftLinker::new("11.0")
        .with_package("swift-lib", "./swift-lib/")
        .link();
}
