#[cfg(target_os = "macos")]
use swift_rs::SwiftLinker;

fn main() {
    #[cfg(target_os = "macos")]
    SwiftLinker::new("11.0")
        .with_package("LoungyLibrary", "./swift-lib/")
        .link();
}
