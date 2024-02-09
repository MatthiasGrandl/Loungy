use std::time::Duration;

use gpui::*;

use crate::{assets::Assets, db::Db, paths::Paths, theme::Theme, workspace::Workspace};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};

pub static WIDTH: f64 = 800.0;
pub static HEIGHT: f64 = 450.0;

fn window_options(center: Point<GlobalPixels>) -> WindowOptions {
    let x: GlobalPixels = center.x - GlobalPixels::from(WIDTH / 2.0);
    let y: GlobalPixels = center.y - GlobalPixels::from(HEIGHT / 2.0);
    let mut options = WindowOptions::default();
    let bounds: Bounds<GlobalPixels> = Bounds::new(
        Point { x, y },
        Size {
            width: GlobalPixels::from(WIDTH),
            height: GlobalPixels::from(HEIGHT),
        },
    );
    options.bounds = WindowBounds::Fixed(bounds);
    options.center = true;
    options.focus = true;
    options.titlebar = None;
    options.is_movable = false;
    options.kind = WindowKind::PopUp;
    options
}
pub struct Window {}

impl Global for Window {}

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
        cx.set_global(Window {});
        Paths::init(cx);
        Db::init(cx);
        Theme::init(cx);
        // TODO: This still only works for a single display
        let center = cx
            .displays()
            .first()
            .expect("No Display found")
            .bounds()
            .center();
        cx.open_window(window_options(center), |cx| {
            cx.spawn(|mut cx| async move {
                loop {
                    if let Ok(event) = receiver.try_recv() {
                        if event.state == global_hotkey::HotKeyState::Released {
                            _ = cx.update_global::<Theme, _>(|_, cx| {
                                cx.activate_window();
                                // query.inner.update(cx, |model, cx| {
                                //     model.select_all(cx);
                                // });
                            });
                        }
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                }
            })
            .detach();

            Workspace::build(cx)
        });
    });
}
