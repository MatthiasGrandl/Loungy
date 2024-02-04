use gpui::*;

use crate::{
    query::{Query, TextInput},
    root::RootCommand,
    theme::Theme,
};

pub struct Workspace {
    query: TextInput,
    command: RootCommand,
    //pub child: Component,
}

pub struct GlobalWorkspace {
    pub view: View<Workspace>,
}

impl Workspace {
    pub fn build(cx: &mut WindowContext) {
        let view = cx.new_view(|cx| Workspace {
            query: TextInput::new(cx, String::from("Hello, world!")),
            command: RootCommand::new(cx),
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
            .child(div().child(self.command.clone()).p_2())
            //.child(self.child.clone())
            .child(div().mt_auto().bg(theme.mantle).w_full().h_10())
    }
}

impl Global for GlobalWorkspace {}
