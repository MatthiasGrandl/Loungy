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

use std::fmt;

use crate::wasm::bindings::loungy::command::shared;
use gpui::SharedString;

fn to_kebap(s: &str) -> String {
    s.chars().fold(String::new(), |mut s, c| {
        if c.is_uppercase() || c.is_numeric() {
            if !s.is_empty() {
                s.push('-');
            }
            s.push(c.to_ascii_lowercase());
        } else {
            s.push(c);
        }
        s
    })
}

impl shared::Icon {
    pub fn path(&self) -> SharedString {
        let name = to_kebap(self.to_string().as_str());
        SharedString::from(format!("icons/{}.svg", name))
    }
}

impl fmt::Display for shared::Icon {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
