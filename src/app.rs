use gpui::*;

use crate::{
    assets::Assets,
    commands::RootCommands,
    db::Db,
    hotkey::HotkeyManager,
    paths::Paths,
    theme::Theme,
    window::{Window, WindowStyle},
    workspace::Workspace,
};

pub fn run_app(app: gpui::App) {
    app.with_assets(Assets).run(move |cx: &mut AppContext| {
        Paths::init(cx);
        Db::init(cx);
        Theme::init(cx);
        // TODO: This still only works for a single display
        let bounds = cx.displays().first().expect("No Display found").bounds();
        cx.open_window(WindowStyle::Main.options(bounds.clone()), |cx| {
            RootCommands::init(cx);
            HotkeyManager::init(cx);
            let view = Workspace::build(cx);
            Window::init(cx);

            view
        });
    });
}
