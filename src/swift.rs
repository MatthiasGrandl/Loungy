use crate::window::Window;
use gpui::Keystroke;
use gpui::WindowContext;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use swift_rs::{swift, Bool, SRData, SRObject, SRString};

#[repr(C)]
#[cfg(target_os = "macos")]
pub struct AppData {
    pub id: SRString,
    pub name: SRString,
}

// Function to fetch application names and icons
#[cfg(target_os = "macos")]
swift!(pub fn get_application_data(cache_dir: &SRString, input: &SRString) -> Option<SRObject<AppData>>);

// Function to fetch application names and icons
#[cfg(target_os = "macos")]
swift!(pub fn get_frontmost_application_data() -> Option<SRObject<AppData>>);

#[cfg(target_os = "macos")]
swift!(pub fn paste(value: SRString, formatting: Bool));

#[cfg(target_os = "macos")]
swift!(pub fn copy_file(path: SRString));

#[cfg(target_os = "macos")]
swift!(pub fn paste_file(path: SRString));

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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
#[cfg(target_os = "macos")]
swift!(pub fn autofill(value: SRString, password: Bool, prev: &SRString) -> Option<SRString>);

#[derive(Deserialize)]
#[allow(dead_code)]
#[cfg(target_os = "macos")]
pub struct MenuItem {
    pub path: Vec<String>,
    #[serde(alias = "pathIndices")]
    pub path_indices: Option<Value>,
    pub shortcut: Option<Keystroke>,
}

// Function to list menu items
#[cfg(target_os = "macos")]
swift!(pub fn menu_items() -> SRData);

// Function to click a menu item
#[cfg(target_os = "macos")]
swift!(pub fn menu_item_select(data: SRData));
