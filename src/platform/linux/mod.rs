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

mod desktop_file;

use crate::components::shared::{Icon, Img};
use crate::paths::paths;

use std::fs;
use std::path::PathBuf;

use super::AppData;

pub fn get_application_data(path: &PathBuf) -> Option<AppData> {
    let cache_dir = paths().cache.join("apps");
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir.clone()).unwrap();
    }
    let cache = cache_dir.to_string_lossy().to_string();
    let last = path.components().last();
    if last.is_none() {
        return None;
    }

    let file_name = last.unwrap().as_os_str().to_string_lossy().to_string();

    let file = desktop_file::ApplicationDesktopFile::try_from(path).ok()?;
    let icon_url: Option<PathBuf> = file.resolve_icon();

    let icon_img = if let Some(icon) = icon_url.clone() {
        Img::default().file(icon)
    } else {
        Img::default().icon(Icon::AppWindow)
    };

    Some(AppData {
        id: file_name.clone(),
        name: file.name.clone(),
        icon: icon_img,
        icon_path: icon_url.unwrap_or_else(|| PathBuf::new()),
        keywords: file.keywords,
        tag: "Application".to_string(),
    })
}

pub fn get_applications_folders() -> Vec<PathBuf> {
    return vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/home")
            .join(whoami::username())
            .join(".local/share/applications"),
    ];
}

pub fn get_frontmost_application_data() -> Option<AppData> {
    None
}
