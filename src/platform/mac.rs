use crate::components::shared::Img;
use crate::paths::paths;
use crate::window::Window;
use gpui::WindowContext;
use std::fs;
use std::path::PathBuf;
use swift_rs::{swift, Bool, SRObject, SRString};

use super::AppData;

#[repr(C)]
struct AppDataMac {
    id: SRString,
    name: SRString,
}

// Function to fetch application names and icons
swift!( fn get_application_data(cache_dir: &SRString, input: &SRString) -> Option<SRObject<AppDataMac>>);

pub fn get_app_data(path: &PathBuf) -> Option<AppData> {
    let cache_dir = paths().cache.join("apps");
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir.clone()).unwrap();
    }
    let cache = cache_dir.to_string_lossy().to_string();
    let path = path.to_string_lossy().to_string();
    unsafe {
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
            icon: Img::list_file(icon_path.clone()),
            icon_path,
        }
    })
}

// Function to fetch application names and icons
swift!( fn get_frontmost_application_data(cache_dir: &SRString) -> Option<SRObject<AppDataMac>>);

pub fn get_focused_app_data() -> Option<AppData> {
    let cache_dir = paths().cache.join("apps");
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir.clone()).unwrap();
    }
    let cache = cache_dir.to_string_lossy().to_string();
    unsafe { get_frontmost_application_data(&SRString::from(cache.as_str())) }.map(|data| {
        let icon_path = cache_dir.join(format!("{}.png", data.id));
        AppData {
            id: data.id.to_string(),
            name: data.name.to_string(),
            icon: Img::list_file(icon_path.clone()),
            icon_path,
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
        unsafe {
            paste(SRString::from(value.as_str()), Bool::from(formatting));
        }
    })
    .detach();
}

pub fn close_and_paste_file(path: &PathBuf, cx: &mut WindowContext) {
    Window::close(cx);
    let path = path.to_string_lossy().to_string();
    cx.spawn(move |mut cx| async move {
        Window::wait_for_close(&mut cx).await;
        unsafe {
            paste_file(SRString::from(path.as_str()));
        }
    })
    .detach();
}

// Function to wait for an input element to be focused and then using AX to fill it
swift!( fn autofill(value: SRString, password: Bool, prev: SRString) -> Option<SRString>);

pub fn auto_fill(value: &str, password: bool, prev: &str) -> Option<String> {
    unsafe {
        autofill(
            SRString::from(value),
            Bool::from(password),
            SRString::from(prev),
        )
    }
    .map(|s| s.to_string())
}

swift!( fn ocr(path: SRString));

pub fn get_text_from_image(path: &PathBuf) {
    unsafe { ocr(SRString::from(path.to_string_lossy().to_string().as_str())) }
}
