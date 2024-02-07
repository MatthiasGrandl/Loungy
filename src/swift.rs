use serde::Deserialize;
use serde_json::Value;
use swift_rs::{swift, SRData, SRObject, SRString};

#[repr(C)]
pub struct AppData {
    pub id: SRString,
    pub name: SRString,
}

// Function to fetch application names and icons
swift!(pub fn get_application_data(cache_dir: &SRString, input: &SRString) -> Option<SRObject<AppData>>);

// Function to emulate typing a string to the foreground app
swift!(pub fn keytap(value: SRString));

// Function to wait for an input element to be focused and then using AX to fill it
swift!(pub fn autofill(value: SRString, prev: SRString) -> Option<SRString>);

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct MenuItem {
    pub path: Vec<String>,
    #[serde(alias = "pathIndices")]
    pub path_indices: Option<Value>,
    // TODO: change this to keystroke format
    //shortcut: Option<Shortcut>,
}

// Function to list menu items
swift!(pub fn menu_items() -> SRData);

// Function to click a menu item
swift!(pub fn menu_item_select(data: SRData));
