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
            origin: Point::new(GlobalPixels::from(0.0), GlobalPixels::from(0.0)),
            size: Size {
                width: GlobalPixels::from(1920.0),
                height: GlobalPixels::from(1080.0),
            },
        });
        cx.open_window(WindowStyle::Main.options(bounds.clone()), |cx| {
            RootCommands::init(cx);
            HotkeyManager::init(cx);
            let view = Workspace::build(cx);
            Window::init(cx);

            view
        });
    });
}
