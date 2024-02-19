use std::path::PathBuf;

use gpui::{AppContext, Global};

pub struct Paths {
    pub cache: PathBuf,
    pub config: PathBuf,
    pub data: PathBuf,
}

impl Global for Paths {}

pub static NAME: &str = "loungy";

impl Paths {
    pub fn init(cx: &mut AppContext) {
        let username = whoami::username();
        #[cfg(target_os = "macos")]
        let user_dir = PathBuf::from("/Users").join(username);
        #[cfg(target_os = "linux")]
        let user_dir = PathBuf::from("/home").join(username);
        cx.set_global(Self {
            #[cfg(target_os = "macos")]
            cache: user_dir.clone().join("Library/Caches").join(NAME),
            #[cfg(target_os = "linux")]
            cache: user_dir.clone().join(".cache").join(NAME),
            config: user_dir.clone().join(".config").join(NAME),
            #[cfg(target_os = "macos")]
            data: user_dir
                .clone()
                .join("Library/Application Support")
                .join(NAME),
            #[cfg(target_os = "linux")]
            data: user_dir.clone().join(".local/share").join(NAME),
        })
    }
}
