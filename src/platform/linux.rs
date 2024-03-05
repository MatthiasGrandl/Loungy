use crate::components::shared::{Icon, Img};
use crate::paths::paths;

use std::fs;
use std::path::PathBuf;

use super::AppData;

pub fn get_app_data(path: &PathBuf) -> Option<AppData> {
    let cache_dir = paths().cache.join("apps");
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir.clone()).unwrap();
    }
    let cache = cache_dir.to_string_lossy().to_string();
    let name = path
        .components()
        .last()
        .unwrap()
        .as_os_str()
        .to_string_lossy()
        .to_string();

    Some(AppData {
        id: name.clone(),
        name: name.clone(),
        icon: Img::list_icon(Icon::AppWindow, None),
        icon_path: PathBuf::new(),
        keywords: vec![],
        tag: "Application".to_string(),
    })
}
