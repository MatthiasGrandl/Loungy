use app::run_app;
use gpui::App;

mod app;
mod assets;
mod commands;
mod components;
mod db;
mod hotkey;
mod paths;
mod query;
mod state;
mod swift;
mod theme;
mod window;
mod workspace;

#[tokio::main]
async fn main() {
    env_logger::init();
    let app = App::new();

    run_app(app)
}
