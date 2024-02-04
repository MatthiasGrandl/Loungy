use std::{thread, time::Duration};

use gpui::*;

use crate::{theme::Theme, workspace::Workspace};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};

fn window_options() -> WindowOptions {
    let mut options = WindowOptions::default();
    let bounds: Bounds<GlobalPixels> = Bounds::new(
        Point {
            x: GlobalPixels::from(500.0),
            y: GlobalPixels::from(320.0),
        },
        Size {
            width: GlobalPixels::from(800.0),
            height: GlobalPixels::from(450.0),
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
    let hotkey = HotKey::new(Some(Modifiers::all()), Code::Space);
    manager.register(hotkey).unwrap();
    let receiver = GlobalHotKeyEvent::receiver().clone();

    app.run(move |cx: &mut AppContext| {
        cx.set_global(Window {});
        Theme::init(cx);
        cx.open_window(window_options(), |cx| {
            cx.spawn(|mut cx| async move {
                loop {
                    if let Ok(event) = receiver.try_recv() {
                        if event.state == global_hotkey::HotKeyState::Released {
                            _ = cx.update_global::<Window, _>(|_, cx| {
                                cx.activate_window();
                            });
                        }
                    }
                    eprintln!("tick");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    eprintln!("tock");
                }
            })
            .detach();

            Workspace::build(cx)
        });
    });
}
