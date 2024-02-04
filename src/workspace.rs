use gpui::*;

use crate::{query::TextInput, root::List, theme::Theme};

pub struct Workspace {
    query: TextInput,
    command: View<List>,
    //pub child: Component,
}

pub struct GlobalWorkspace {
    pub view: View<Workspace>,
}

impl Workspace {
    pub fn build(cx: &mut WindowContext) {
        let view = cx.new_view(|cx| Workspace {
            query: TextInput::new(cx, String::from("Hello, world!")),
            command: List::new(cx),
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
            //.rounded_xl()
            //.border_2()
            //.border_color(theme.crust)
            .text_color(theme.text)
            .child(self.query.clone())
            .child(div().child(self.command.clone()).p_2())
            .child(
                div()
                    .mt_auto()
                    .bg(theme.mantle)
                    .w_full()
                    .h_8()
                    .border_t_1()
                    .border_color(theme.crust),
            )
    }
}

impl Global for GlobalWorkspace {}
