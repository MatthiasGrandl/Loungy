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

use crate::components::shared::Img;
use crate::paths::paths;
use crate::window::Window;
use cocoa::appkit::NSPasteboard;
use gpui::{AsyncWindowContext, WindowContext};
use std::time::Duration;
use std::{
    fs,
    path::{Path, PathBuf},
};
use swift_rs::{swift, Bool, SRObject, SRString};

use super::{AppData, ClipboardWatcher};

#[repr(C)]
struct AppDataMac {
    id: SRString,
    name: SRString,
}

pub fn get_application_data(path: &Path) -> Option<AppData> {
    let cache_dir = paths().cache.join("apps");
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir.clone()).unwrap();
    }
    let cache = cache_dir.to_string_lossy().to_string();
    let extension = match path.extension() {
        Some(ext) => ext,
        None => return None,
    };
    let ex = extension.to_str().unwrap() == "appex";
    let tag = match ex {
        true => "System Setting",
        false => "Application",
    };
    let path = path.to_string_lossy().to_string();
    unsafe {
        swift!( fn get_application_data(cache_dir: &SRString, input: &SRString) -> Option<SRObject<AppDataMac>>);
        get_application_data(
            &SRString::from(cache.as_str()),
            &SRString::from(path.as_str()),
        )
    }
    .map(|data| {
        let icon_path = cache_dir.join(format!("{}.png", data.id));

        AppData {
            id: data.id.to_string(),
            name: data.name.to_string(),
            icon: Img::default().file(icon_path.clone()),
            icon_path,
            keywords: vec![],
            tag: tag.to_string(),
        }
    })
}

pub fn get_application_folders() -> Vec<PathBuf> {
    let user_dir = PathBuf::from("/Users")
        .join(whoami::username())
        .join("Applications");
    let mut user_dirs = user_dir
        .read_dir()
        .map(|i| {
            i.into_iter()
                .filter_map(|dir| {
                    if let Ok(dir) = dir {
                        let path = dir.path();
                        if path.is_dir() {
                            return Some(path);
                        }
                    }
                    None
                })
                .collect::<Vec<PathBuf>>()
        })
        .unwrap_or_default();
    user_dirs.append(&mut vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/Applications/Chromium Apps"),
        PathBuf::from("/System/Applications/Utilities"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Library/CoreServices/Applications"),
        PathBuf::from("/Library/PreferencePanes"),
        PathBuf::from("/System/Library/ExtensionKit/Extensions"),
        PathBuf::from("/System/Library/CoreServices/Finder.app"),
        user_dir.clone(),
        user_dir.clone().join("Home Manager Apps"),
        user_dir.clone().join("Chromium Apps.localized"),
        user_dir.clone().join("Chrome Apps.localized"),
        user_dir.clone().join("Brave Apps.localized"),
    ]);
    user_dirs
}

pub fn get_application_files() -> Vec<PathBuf> {
    let mut files = Vec::new();

    for applications_folder in get_application_folders() {
        let dir = applications_folder.read_dir();
        if dir.is_err() {
            continue;
        }
        if let Some(ext) = applications_folder.extension() {
            if ext.eq("app") {
                files.push(applications_folder);
            }
        } else {
            for entry in dir.unwrap().flatten() {
                let path = entry.path();
                files.push(path);
            }
        }
    }

    files
}

pub fn get_frontmost_application_data() -> Option<AppData> {
    let cache_dir = paths().cache.join("apps");
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir.clone()).unwrap();
    }
    let cache = cache_dir.to_string_lossy().to_string();
    swift!( fn get_frontmost_application_data(cache_dir: &SRString) -> Option<SRObject<AppDataMac>>);
    unsafe { get_frontmost_application_data(&SRString::from(cache.as_str())) }.map(|data| {
        let icon_path = cache_dir.join(format!("{}.png", data.id));
        AppData {
            id: data.id.to_string(),
            name: data.name.to_string(),
            icon: Img::default().file(icon_path.clone()),
            icon_path,
            keywords: vec![],
            tag: "".to_string(),
        }
    })
}

swift!( fn paste(value: SRString, formatting: Bool));

swift!( fn copy_file(path: SRString));

swift!( fn paste_file(path: SRString));

pub fn close_and_paste(value: &str, formatting: bool, cx: &mut WindowContext) {
    Window::close(cx);
    let value = value.to_string();
    cx.spawn(move |mut cx| async move {
        Window::wait_for_close(&mut cx).await;
        ClipboardWatcher::disabled(&mut cx);
        unsafe {
            paste(SRString::from(value.as_str()), Bool::from(formatting));
        }
    })
    .detach();
}

pub fn close_and_paste_file(path: &Path, cx: &mut WindowContext) {
    Window::close(cx);
    let path = path.to_string_lossy().to_string();
    cx.spawn(move |mut cx| async move {
        Window::wait_for_close(&mut cx).await;
        ClipboardWatcher::disabled(&mut cx);
        unsafe {
            paste_file(SRString::from(path.as_str()));
        }
    })
    .detach();
}

// Function to wait for an input element to be focused and then using AX to fill it
pub fn autofill(value: &str, password: bool, prev: &str) -> Option<String> {
    unsafe {
        swift!( fn autofill(value: SRString, password: Bool, prev: SRString) -> Option<SRString>);
        autofill(
            SRString::from(value),
            Bool::from(password),
            SRString::from(prev),
        )
    }
    .map(|s| s.to_string())
}

pub fn ocr(path: &Path) {
    swift!( fn ocr(path: SRString));
    unsafe { ocr(SRString::from(path.to_string_lossy().to_string().as_str())) }
}

pub async fn clipboard(
    mut on_change: impl FnMut(&mut AsyncWindowContext),
    mut cx: AsyncWindowContext,
) {
    unsafe {
        let pasteboard = NSPasteboard::generalPasteboard(cocoa::base::nil);
        let mut change_count = pasteboard.changeCount();

        loop {
            if pasteboard.changeCount() != change_count {
                change_count = pasteboard.changeCount();
                on_change(&mut cx);
            }
            cx.background_executor()
                .timer(Duration::from_millis(50))
                .await;
        }
    }
}
