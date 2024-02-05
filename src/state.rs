use gpui::*;

use crate::{
    commands::root::list::RootBuilder,
    query::{TextInput, TextModel},
};

pub struct StateItem {
    pub query: TextInput,
    pub view: AnyView,
}

impl StateItem {
    pub fn init(view: impl StateViewBuilder, cx: &mut WindowContext) -> Self {
        let query = TextInput::new(cx, String::from(""));
        let view = view.build(&query.model, cx);
        Self { query, view }
    }
}

pub trait StateViewBuilder {
    fn build(&self, query: &Model<TextModel>, cx: &mut WindowContext) -> AnyView;
}

pub struct State {
    pub stack: Vec<StateItem>,
}

pub struct StateModel {
    pub inner: Model<State>,
}

impl StateModel {
    pub fn init(cx: &mut WindowContext) -> Self {
        let item = StateItem::init(RootBuilder {}, cx);
        let state = cx.new_model(|cx| State { stack: vec![item] });
        Self { inner: state }
    }
    pub fn pop(&self, cx: &mut WindowContext) {
        self.inner.update(cx, |model, cx| {
            if model.stack.len() > 1 {
                model.stack.pop();
                cx.notify();
            };
        });
    }
    pub fn push(&self, view: impl StateViewBuilder, cx: &mut WindowContext) {
        let item = StateItem::init(view, cx);
        self.inner.update(cx, |model, cx| {
            model.stack.push(item);
            cx.notify();
        });
    }
}

impl Global for State {}
