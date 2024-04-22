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

use gpui::*;
use log::*;
use serde::{Deserialize, Serialize};

use crate::{db::db, paths::paths};

fn color_to_hsla(color: catppuccin::Colour) -> Hsla {
    Rgba {
        r: color.0 as f32 / 255.0,
        g: color.1 as f32 / 255.0,
        b: color.2 as f32 / 255.0,
        a: 1.0,
    }
    .into()
}

impl From<catppuccin::Flavour> for Theme {
    fn from(flavor: catppuccin::Flavour) -> Self {
        let colors = flavor.colours();
        let name = flavor.name();
        // name capitalized
        let name = name
            .chars()
            .next()
            .unwrap()
            .to_uppercase()
            .collect::<String>()
            + &name[1..];
        Self {
            name: format!("Catppuccin {}", name).into(),
            font_sans: "Inter".into(),
            font_mono: "JetBrains Mono".into(),
            window_background: Some(WindowBackgroundAppearanceContent::Blurred { opacity: 0.9 }),
            flamingo: color_to_hsla(colors.flamingo),
            pink: color_to_hsla(colors.pink),
            mauve: color_to_hsla(colors.mauve),
            red: color_to_hsla(colors.red),
            maroon: color_to_hsla(colors.maroon),
            peach: color_to_hsla(colors.peach),
            yellow: color_to_hsla(colors.yellow),
            green: color_to_hsla(colors.green),
            teal: color_to_hsla(colors.teal),
            sky: color_to_hsla(colors.sky),
            sapphire: color_to_hsla(colors.sapphire),
            blue: color_to_hsla(colors.blue),
            lavender: color_to_hsla(colors.lavender),
            text: color_to_hsla(colors.text),
            subtext1: color_to_hsla(colors.subtext1),
            subtext0: color_to_hsla(colors.subtext0),
            overlay2: color_to_hsla(colors.overlay2),
            overlay1: color_to_hsla(colors.overlay1),
            overlay0: color_to_hsla(colors.overlay0),
            surface2: color_to_hsla(colors.surface2),
            surface1: color_to_hsla(colors.surface1),
            surface0: color_to_hsla(colors.surface0),
            base: color_to_hsla(colors.base),
            mantle: color_to_hsla(colors.mantle),
            crust: color_to_hsla(colors.crust),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Theme {
    pub name: SharedString,
    pub font_sans: SharedString,
    pub font_mono: SharedString,
    pub window_background: Option<WindowBackgroundAppearanceContent>,
    pub flamingo: Hsla,
    pub pink: Hsla,
    pub mauve: Hsla,
    pub red: Hsla,
    pub maroon: Hsla,
    pub peach: Hsla,
    pub yellow: Hsla,
    pub green: Hsla,
    pub teal: Hsla,
    pub sky: Hsla,
    pub sapphire: Hsla,
    pub blue: Hsla,
    pub lavender: Hsla,
    pub text: Hsla,
    pub subtext1: Hsla,
    pub subtext0: Hsla,
    pub overlay2: Hsla,
    pub overlay1: Hsla,
    pub overlay0: Hsla,
    pub surface2: Hsla,
    pub surface1: Hsla,
    pub surface0: Hsla,
    pub base: Hsla,
    pub mantle: Hsla,
    pub crust: Hsla,
}

fn load_fonts(cx: &mut AppContext) -> gpui::Result<()> {
    let font_paths = cx.asset_source().list("fonts")?;
    let mut embedded_fonts = Vec::new();
    for font_path in font_paths {
        if font_path.ends_with(".ttf") {
            let font_bytes = cx.asset_source().load(&font_path)?;
            embedded_fonts.push(font_bytes);
        }
    }
    cx.text_system().add_fonts(embedded_fonts)
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum WindowBackgroundAppearanceContent {
    Blurred {
        opacity: f32,
    },
    Transparent {
        opacity: f32,
    },
    #[default]
    Opaque,
}

impl From<WindowBackgroundAppearanceContent> for WindowBackgroundAppearance {
    fn from(content: WindowBackgroundAppearanceContent) -> Self {
        match content {
            WindowBackgroundAppearanceContent::Blurred { .. } => {
                WindowBackgroundAppearance::Blurred
            }
            WindowBackgroundAppearanceContent::Transparent { .. } => {
                WindowBackgroundAppearance::Transparent
            }
            WindowBackgroundAppearanceContent::Opaque => WindowBackgroundAppearance::Opaque,
        }
    }
}

impl WindowBackgroundAppearanceContent {
    pub fn opacity(&self) -> f32 {
        match self {
            WindowBackgroundAppearanceContent::Blurred { opacity }
            | WindowBackgroundAppearanceContent::Transparent { opacity } => *opacity,
            WindowBackgroundAppearanceContent::Opaque => 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ThemeSettings {
    pub light: String,
    pub dark: String,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            light: "Catppuccin Latte".into(),
            dark: "Catppuccin Mocha".into(),
        }
    }
}

impl Theme {
    pub fn init(cx: &mut AppContext) {
        load_fonts(cx).expect("Failed to load fonts");
        let appearance = cx.window_appearance();
        let theme = Theme::mode(appearance);

        cx.set_global(theme);
    }
    pub fn mode(mode: WindowAppearance) -> Theme {
        let settings = db().get::<ThemeSettings>("theme").unwrap_or_default();
        let list = Theme::list();
        let name = match mode {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => settings.dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => settings.light,
        };
        list.clone()
            .into_iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| {
                error!("Theme not found: {}", name);
                list.first().unwrap().clone()
            })
            .clone()
    }

    pub fn list() -> Vec<Theme> {
        let config = paths().config.clone().join("themes");
        let mut user_themes: Vec<Theme> = match std::fs::read_dir(config) {
            Ok(themes) => themes
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    let theme: Theme = match std::fs::read_to_string(path) {
                        Ok(theme) => match toml::from_str(&theme) {
                            Ok(theme) => theme,
                            Err(e) => {
                                error!("Failed to parse theme: {}", e);
                                return None;
                            }
                        },
                        Err(e) => {
                            error!("Failed to read theme: {}", e);
                            return None;
                        }
                    };
                    Some(theme)
                })
                .collect(),
            Err(e) => {
                error!("Failed to read themes: {}", e);
                vec![]
            }
        };
        let mut themes: Vec<Theme> = catppuccin::Flavour::all()
            .into_iter()
            .map(Self::from)
            .collect();
        themes.append(&mut user_themes);

        themes
    }
}

impl Global for Theme {}
