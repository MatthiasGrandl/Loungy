use gpui::{AppContext, KeyBinding};

use self::query::{MoveDown, MoveUp};

pub mod query {
    use gpui::actions;

    actions!(gpui, [MoveUp, MoveDown, Input]);
}

pub fn register(cx: &mut AppContext) {
    cx.bind_keys(vec![
        KeyBinding::new("down", MoveDown, Some("query")),
        KeyBinding::new("up", MoveUp, Some("query")),
    ]);
}
