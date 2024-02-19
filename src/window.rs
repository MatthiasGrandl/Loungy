use std::time::Duration;

use async_std::task::sleep;
use gpui::*;

use crate::state::StateModel;

pub static WIDTH: f64 = 800.0;
pub static HEIGHT: f64 = 450.0;

pub enum WindowStyle {
    Main,
    Toast { width: f64, height: f64 },
}

impl WindowStyle {
    pub fn options(&self, bounds: Bounds<GlobalPixels>) -> WindowOptions {
        let mut options = WindowOptions::default();
        let center = bounds.center();

        let (width, height, x, y) = match self {
            WindowStyle::Main => {
                options.focus = true;
                let width = GlobalPixels::from(WIDTH);
                let height = GlobalPixels::from(HEIGHT);
                let x: GlobalPixels = center.x - width / 2.0;
                let y: GlobalPixels = center.y - height / 2.0;
                (width, height, x, y)
            }
            WindowStyle::Toast { width, height } => {
                options.focus = false;
                let width = GlobalPixels::from(*width);
                let height = GlobalPixels::from(*height);
                let x: GlobalPixels = center.x - width / 2.0;
                let y: GlobalPixels = bounds.bottom() - height - GlobalPixels::from(200.0);
                (width, height, x, y)
            }
        };
        let bounds: Bounds<GlobalPixels> = Bounds::new(Point { x, y }, Size { width, height });
        options.bounds = WindowBounds::Fixed(bounds);
        options.titlebar = None;
        options.is_movable = false;
        options.kind = WindowKind::PopUp;
        options
    }
}

pub struct Window {
    //inner: View<Workspace>,
    hidden: bool,
}

impl Window {
    pub fn init(cx: &mut AppContext) {
        cx.set_global::<Self>(Self {
            //inner: view.clone(),
            hidden: false,
        });
    }
    pub fn open(cx: &mut WindowContext) {
        let _ = cx.update_global::<Self, _>(|this, cx| {
            if this.hidden {
                cx.activate_window();
                this.hidden = false;
            }
        });
    }
    pub fn close(cx: &mut WindowContext) {
        cx.update_global::<Self, _>(|this, cx| {
            this.hidden = true;
            cx.hide();
        });
        // After 90 seconds, reset the state
        cx.spawn(|mut cx| async move {
            sleep(Duration::from_millis(90)).await;
            // cx.background_executor()
            //     .timer(Duration::from_secs(90))
            //     .await;
            let _ = cx.update_global::<Self, _>(|window, cx| {
                if window.hidden {
                    cx.update_global::<StateModel, _>(|model, cx| {
                        model.reset(cx);
                    });
                }
            });
        })
        .detach();
    }
}

impl Global for Window {}
