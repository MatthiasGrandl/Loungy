use gpui::Keystroke;
use serde::Deserialize;
use serde_json::Value;
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

// Function to emulate typing a string to the foreground app
#[cfg(target_os = "macos")]
swift!(pub fn keytap(value: SRString));

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
