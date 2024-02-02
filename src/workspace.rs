use gpui::*;

use crate::{query::TextInput, theme::Theme};

pub struct Workspace {
    query: TextInput,
}

impl Workspace {
    pub fn build(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(|cx| Self {
            query: TextInput::new(cx, String::from("Hello, world!")),
        })
    }
}

impl Render for Workspace {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        div()
            .full()
            .flex()
            .flex_col()
            .bg(theme.base)
            .text_color(theme.text)
            .child(self.query.clone())
    }
}
