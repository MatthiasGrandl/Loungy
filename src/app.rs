use gpui::*;

use crate::{theme::Theme, workspace::Workspace};

pub fn run_app(app: gpui::App) {
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

    app.run(|cx: &mut AppContext| {
        Theme::init(cx);

        cx.open_window(options, |cx| Workspace::build(cx));
    });
}
