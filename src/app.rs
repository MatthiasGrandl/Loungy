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

use crate::{
    assets::Assets,
    commands::RootCommands,
    hotkey::HotkeyManager,
    theme::Theme,
    window::{Window, WindowStyle},
    workspace::Workspace,
};

pub fn run_app(app: gpui::App) {
    app.with_assets(Assets).run(move |cx: &mut AppContext| {
        Theme::init(cx);
        // TODO: This still only works for a single display
        let bounds = cx.displays().first().map(|d| d.bounds()).unwrap_or(Bounds {
            origin: Point::new(Pixels::from(0.0), Pixels::from(0.0)),
            size: Size {
                width: Pixels::from(1920.0),
                height: Pixels::from(1080.0),
            },
        });
        let _ = cx.open_window(WindowStyle::Main.options(bounds), |cx| {
            let theme = cx.global::<Theme>();
            cx.set_background_appearance(WindowBackgroundAppearance::from(
                theme.window_background.clone().unwrap_or_default(),
            ));
            RootCommands::init(cx);
            HotkeyManager::init(cx);
            let view = Workspace::build(cx);
            Window::init(cx);

            view
        });
    });
}
