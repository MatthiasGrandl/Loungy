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
use gpui::{AppContext, AsyncAppContext, Global};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
pub use mac::*;

#[derive(Clone)]
pub struct AppData {
    pub id: String,
    pub name: String,
    pub icon: Img,
    pub icon_path: PathBuf,
    pub keywords: Vec<String>,
    pub tag: String,
}

pub struct ClipboardWatcher {
    enabled: bool,
}
impl ClipboardWatcher {
    pub fn init(cx: &mut AppContext) {
        cx.set_global(Self { enabled: true });
    }
    pub fn enabled(cx: &mut AsyncAppContext) {
        let _ = cx.update_global::<Self, _>(|this, _| {
            this.enabled = true;
        });
    }
    pub fn disabled(cx: &mut AsyncAppContext) {
        let _ = cx.update_global::<Self, _>(|this, _| {
            this.enabled = false;
        });
    }
    pub fn is_enabled(cx: &AsyncAppContext) -> bool {
        cx.try_read_global::<Self, _>(|x, _| x.enabled)
            .unwrap_or(false)
    }
}
impl Global for ClipboardWatcher {}
