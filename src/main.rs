use app::run_app;
use gpui::App;

mod app;
mod assets;
mod commands;
mod icon;
mod lazy;
mod list;
mod nucleo;
mod paths;
mod query;
mod theme;
mod workspace;

fn main() {
    let app = App::new();

    run_app(app)
}
