use gpui::*;

use crate::{query::TextInput, theme::Theme};

#[derive(Clone, IntoElement)]
pub enum Component {
    List { items: Vec<String> },
    Text { text: String },
    None,
}

impl RenderOnce for Component {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        match self {
            Component::List { items } => div().child("test"),
            Component::Text { text } => div().child(text),
            Component::None => div().child("empty"),
        }
        .p_4()
    }
}

#[derive(Clone)]
pub struct Workspace {
    query: TextInput,
    pub component: Component,
}

pub struct GlobalWorkspace {
    pub view: View<Workspace>,
}

impl Workspace {
    pub fn build(cx: &mut WindowContext) {
        let view = cx.new_view(|cx| Workspace {
            query: TextInput::new(cx, String::from("Hello, world!")),
            component: Component::None,
        });
        cx.set_global(GlobalWorkspace { view });
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
            .child(self.component.clone())
            .child(div().mt_auto().bg(theme.mantle).w_full().h_10())
    }
}

impl Global for GlobalWorkspace {}
