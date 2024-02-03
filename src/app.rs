use std::time::Duration;

use gpui::*;

use crate::{
    query::Query,
    theme::Theme,
    workspace::{GlobalWorkspace, Workspace},
};
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

pub fn run_app(app: gpui::App) {
    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::all()), Code::Space);
    manager.register(hotkey).unwrap();
    let receiver = GlobalHotKeyEvent::receiver().clone();

    app.run(move |cx: &mut AppContext| {
        cx.spawn(|cx| async move {
            eprintln!("Hotkey listener started");
            loop {
                if let Ok(event) = receiver.try_recv() {
                    if event.state == global_hotkey::HotKeyState::Released {
                        let _ = cx.open_window(window_options(), |cx| {
                            let gw = cx.global::<GlobalWorkspace>();
                            gw.view.clone()
                        });
                    }
                }
                //eprintln!("loop1");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .detach();
        cx.set_global(Query {
            inner: String::from(""),
        });
        Theme::init(cx);
        cx.open_window(window_options(), |cx| {
            Workspace::build(cx);
            let gw = cx.global::<GlobalWorkspace>();
            gw.view.clone()
        });
    });
}
