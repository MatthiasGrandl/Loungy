use gpui::*;
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use crate::{state::LazyMutex, theme};

#[derive(Clone)]
pub struct Loader(Arc<AtomicBool>);

impl Loader {
    pub fn add() -> Loader {
        let loader = Loader(Arc::new(AtomicBool::new(true)));
        LOADERS.lock().loaders.push(loader.clone());
        loader
    }
    pub fn remove(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::Relaxed);
        let mut loaders = LOADERS.lock();
        loaders
            .loaders
            .retain(|loader| loader.0.load(std::sync::atomic::Ordering::Relaxed));
    }
}

static LOADERS: LazyMutex<LoaderState> = LazyMutex::new(LoaderState::new);

struct LoaderState {
    show: bool,
    timestamp: Instant,
    loaders: Vec<Loader>,
}

impl LoaderState {
    fn new() -> Self {
        Self {
            show: false,
            timestamp: Instant::now(),
            loaders: Vec::new(),
        }
    }
    fn get(&mut self) -> (bool, Instant) {
        let active = self
            .loaders
            .iter()
            .any(|loader| loader.0.load(std::sync::atomic::Ordering::Relaxed));

        if self.show != active {
            self.show = active;
            self.timestamp = Instant::now();
        }
        (self.show, self.timestamp)
    }
}

#[derive(IntoElement)]
pub struct ActiveLoaders {}
impl RenderOnce for ActiveLoaders {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<theme::Theme>().clone();

        div().w_full().h_px().bg(theme.mantle).relative().child(
            div().absolute().h_full().top_0().bottom_0().with_animation(
                "loader",
                Animation::new(Duration::from_secs(1)).repeat(),
                {
                    move |div, i| {
                        let w = 1.0;
                        let w_start = 0.4;
                        let w_stop = 0.5;

                        let (show, show_ts) = LOADERS.lock().get();

                        let (left, width) = if i > 0.4 {
                            let i = (i - 0.4) / 0.6;
                            let e = linear(i);
                            let left = e * w;
                            let width = e * (w_stop - w_start) + w_start;
                            (left, width)
                        } else {
                            let i = i / 0.4;
                            (0.0, linear(i) * w_start)
                        };
                        let opacity = {
                            let i = show_ts.elapsed().as_millis() as f32 / 500.0;
                            let i = if i > 1.0 { 1.0 } else { i };
                            if show {
                                1.0 - i
                            } else {
                                i
                            }
                        };
                        let mut bg = theme.lavender;

                        bg.fade_out(opacity);
                        div.left(relative(left)).w(relative(width)).bg(bg)
                    }
                },
            ),
        )
    }
}
