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

use freedesktop_entry_parser::{parse_entry, AttrSelector};
use freedesktop_icons::lookup;
use std::path::PathBuf;

pub(crate) struct ApplicationDesktopFile {
    pub name: String,
    pub icon: Option<String>,
    pub keywords: Vec<String>,
}

pub(crate) enum DesktopFileError {
    FileNotFound,
    NoDesktopEntry,
    InvalidFormat,
    HiddenFile,
}

impl ApplicationDesktopFile {
    pub(crate) fn resolve_icon(&self) -> Option<PathBuf> {
        let icon_name = self.icon.as_ref()?;

        lookup(icon_name).with_cache().find()
    }
}

impl TryFrom<&PathBuf> for ApplicationDesktopFile {
    type Error = DesktopFileError;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        let entry = parse_entry(value).map_err(|_| DesktopFileError::InvalidFormat)?;

        let content_section: AttrSelector<&str> = entry.section("Desktop Entry");
        let name = content_section
            .attr("Name")
            .ok_or(DesktopFileError::NoDesktopEntry)?
            .to_string();

        let icon = content_section.attr("Icon").map(|s| s.to_string());

        let keywords = content_section
            .attr("Keywords")
            .map(|s| s.split(';').map(|s| s.to_string()).collect())
            .unwrap_or_default();

        let no_display = content_section
            .attr("NoDisplay")
            .map_or(Ok(false), |s| s.parse::<bool>())
            .map_err(|_| DesktopFileError::InvalidFormat)?;

        if no_display {
            // Hidden files are typically used for window managers and other system utilities
            // so this is not an application we can start
            return Err(DesktopFileError::HiddenFile);
        }

        Ok(ApplicationDesktopFile {
            name,
            icon,
            keywords,
        })
    }
}
