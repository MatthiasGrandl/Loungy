use gpui::*;

use crate::{
    query::{TextInput, TextModel},
    root::List,
    theme::Theme,
};

pub struct Workspace {
    query: TextInput,
    state: Model<StateModel>,
}

impl Workspace {
    pub fn build(cx: &mut WindowContext) -> View<Self> {
        let view = cx.new_view(|cx| {
            let query = TextInput::new(cx, String::from("Hello, world!"));
            cx.set_global::<Query>(Query {
                inner: query.model.clone(),
            });
            let root: AnyView = List::new(cx).into();
            let state = cx.new_model(|_cx| StateModel { root });
            cx.set_global::<State>(State {
                inner: state.clone(),
            });
            Workspace { query, state }
        });
        view
    }
}

pub struct Query {
    pub inner: Model<TextModel>,
}

impl Global for Query {}

pub struct StateModel {
    pub root: AnyView,
}

pub struct State {
    pub inner: Model<StateModel>,
}

impl Global for State {}

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
            .child(div().child(self.state.read(cx).root.clone()).p_2())
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
