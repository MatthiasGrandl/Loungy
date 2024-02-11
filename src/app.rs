use std::time::Duration;

use gpui::*;

use crate::{
    assets::Assets,
    db::Db,
    paths::Paths,
    theme::Theme,
    window::{Window, WindowStyle},
    workspace::Workspace,
};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};

pub fn run_app(app: gpui::App) {
    let manager = GlobalHotKeyManager::new().unwrap();
    let mut mods = Modifiers::empty();

    mods.set(Modifiers::CONTROL, true);
    mods.set(Modifiers::ALT, true);
    mods.set(Modifiers::META, true);
    let hotkey = HotKey::new(Some(mods), Code::Space);
    manager.register(hotkey).unwrap();
    let receiver = GlobalHotKeyEvent::receiver().clone();
    app.with_assets(Assets).run(move |cx: &mut AppContext| {
        Paths::init(cx);
        Db::init(cx);
        Theme::init(cx);
        // TODO: This still only works for a single display
        let bounds = cx.displays().first().expect("No Display found").bounds();
        cx.open_window(WindowStyle::Main.options(bounds.clone()), |cx| {
            let view = Workspace::build(cx);
            Window::init(cx);
            cx.spawn(|mut cx| async move {
                loop {
                    if let Ok(event) = receiver.try_recv() {
                        if event.state == global_hotkey::HotKeyState::Released {
                            Window::open(bounds, &mut cx);
                        }
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                }
            })
            .detach();

            view
        });
    });
}
