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
        let user_dir = PathBuf::from("/Users").join(username);
        cx.set_global(Self {
            cache: user_dir.clone().join("Library/Caches"),
            config: user_dir.clone().join("Library/Preferences"),
            data: user_dir.clone().join("Library/Application Support"),
        })
    }
}
