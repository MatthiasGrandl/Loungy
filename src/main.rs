use app::run_app;
use gpui::*;

mod app;
mod query;
mod theme;
mod workspace;

fn main() {
    run_app(App::new())
}
