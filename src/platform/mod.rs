use crate::components::shared::Img;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
pub use mac::*;

pub struct AppData {
    pub id: String,
    pub name: String,
    pub icon: Img,
    pub icon_path: PathBuf,
    pub keywords: Vec<String>,
    pub tag: String,
}
