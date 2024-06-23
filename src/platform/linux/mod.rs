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

use walkdir::WalkDir;

use crate::components::shared::{Icon, Img};
use crate::paths::paths;

use std::path::PathBuf;
use std::{env, fs};

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

pub fn get_application_folders() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(data_home) = env::var("XDG_DATA_HOME") {
        let data_home = PathBuf::from(data_home);
        if data_home.exists() {
            dirs.push(data_home);
        }
    } else {
        let home_dir = PathBuf::from("/home").join(whoami::username());
        let share_dir = home_dir.join(PathBuf::from("/.local/share"));

        if share_dir.exists() {
            dirs.push(share_dir);
        }
    }

    if let Ok(xdg_data_dirs) = env::var("XDG_DATA_DIRS") {
        dirs.extend(
            xdg_data_dirs
                .split(":")
                .map(|s| PathBuf::from(s))
                .filter(|d| d.exists()),
        );
    } else {
        let usr_local_share = PathBuf::from("/usr/local/share");
        let usr_share = PathBuf::from("/usr/share");

        if usr_share.exists() {
            dirs.push(usr_share);
        }

        if usr_local_share.exists() {
            dirs.push(usr_local_share);
        }
    }

    return dirs;
}

pub fn get_application_files() -> Vec<PathBuf> {
    let dirs = get_application_folders();

    let mut files = Vec::new();
    for dir in dirs {
        let walker = WalkDir::new(dir).into_iter();
        for entry in walker {
            if let Ok(entry) = entry {
                if entry
                    .path()
                    .extension()
                    .and_then(|ext| Some(ext == "desktop"))
                    .unwrap_or(false)
                {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    return files;
}

pub fn get_frontmost_application_data() -> Option<AppData> {
    None
}
