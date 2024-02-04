use app::run_app;
use gpui::App;

mod app;
mod list;
mod query;
mod theme;
mod workspace;

#[tokio::main]
async fn main() {
    let app = App::new();

    run_app(app)
}
