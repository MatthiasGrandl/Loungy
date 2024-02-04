use gpui::*;

use crate::{
    query::{TextInput, TextModel},
    root::List,
    theme::Theme,
};

pub struct Workspace {
    query: TextInput,
    command: View<List>,
    //pub child: Component,
}

impl Workspace {
    pub fn build(cx: &mut WindowContext) -> View<Self> {
        let view = cx.new_view(|cx| {
            let query = TextInput::new(cx, String::from("Hello, world!"));
            cx.set_global::<Query>(Query {
                inner: query.model.clone(),
            });
            Workspace {
                query,
                command: List::new(cx),
            }
        });
        view
    }
}

pub struct Query {
    pub inner: Model<TextModel>,
}

impl Global for Query {}

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
