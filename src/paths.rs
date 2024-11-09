/*
 *
 *  This source file is part of the Loungy open source project
 *
 *  Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 *  Licensed under MIT License
 *
 *  See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 *
 */

use std::{path::PathBuf, sync::OnceLock};

pub struct Paths {
    pub path_env: String,
    pub cache: PathBuf,
    pub config: PathBuf,
    pub data: PathBuf,
}

pub static NAME: &str = "loungy";

impl Paths {
    pub fn new() -> Self {
        let username = whoami::username();
        #[cfg(target_os = "macos")]
        let user_dir = PathBuf::from("/Users").join(username.clone());
        #[cfg(target_os = "linux")]
        let user_dir = PathBuf::from("/home").join(username.clone());
        Self {
            #[cfg(target_os = "macos")]
            path_env: format!(
                "/opt/homebrew/bin:/usr/local/bin:/Users/{}/.nix-profile/bin",
                username
            ),
            #[cfg(target_os = "linux")]
            path_env: format!(
                "/opt/homebrew/bin:/usr/local/bin:/home/{}/.nix-profile/bin",
                username
            ),
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
        }
    }
}

pub fn paths() -> &'static Paths {
    static PATHS: OnceLock<Paths> = OnceLock::new();
    PATHS.get_or_init(Paths::new)
}
