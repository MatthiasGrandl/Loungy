use gpui::*;

use crate::{
    commands::root::list::RootBuilder,
    list::Img,
    query::{TextInput, TextModel},
};

pub struct StateItem {
    pub query: TextInput,
    pub view: AnyView,
    pub actions: ActionsModel,
}

impl StateItem {
    pub fn init(view: impl StateView, cx: &mut WindowContext) -> Self {
        let actions = ActionsModel::init(cx);
        let query = TextInput::new(&actions, cx);
        let view = view.build(&query.model, &actions, cx);
        Self {
            query,
            view,
            actions,
        }
    }
}

pub trait StateView {
    fn build(
        &self,
        query: &Model<TextModel>,
        actions: &ActionsModel,
        cx: &mut WindowContext,
    ) -> AnyView;
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
    pub fn push(&self, view: impl StateView, cx: &mut WindowContext) {
        let item = StateItem::init(view, cx);
        self.inner.update(cx, |model, cx| {
            model.stack.push(item);
            cx.notify();
        });
    }
}

// Actions

pub trait CloneableFn: Fn(&WindowContext) -> () {
    fn clone_box<'a>(&self) -> Box<dyn 'a + CloneableFn>
    where
        Self: 'a;
}

impl<F> CloneableFn for F
where
    F: Fn(&WindowContext) -> () + Clone,
{
    fn clone_box<'a>(&self) -> Box<dyn 'a + CloneableFn>
    where
        Self: 'a,
    {
        Box::new(self.clone())
    }
}

impl<'a> Clone for Box<dyn 'a + CloneableFn> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

#[derive(Clone)]
pub struct Action {
    pub label: String,
    pub shortcut: Option<Keystroke>,
    pub image: Img,
    pub action: Box<dyn CloneableFn>,
}

impl Action {
    pub fn new(
        image: Img,
        label: impl ToString,
        shortcut: Option<Keystroke>,
        action: Box<dyn CloneableFn>,
    ) -> Self {
        Self {
            label: label.to_string(),
            shortcut,
            action,
            image,
        }
    }
}

pub struct Actions {
    global: Vec<Action>,
    local: Vec<Action>,
    combined: Vec<Action>,
}

impl Actions {
    fn compute(&mut self) {
        let mut combined = self.local.clone();
        combined.append(&mut self.global);
        // if there are actions, make the first action the default action
        if let Some(action) = combined.get_mut(0) {
            action.shortcut = Some(Keystroke {
                modifiers: Modifiers::default(),
                key: "enter".to_string(),
                ime_key: None,
            });
        }
        self.combined = combined;
    }
}

#[derive(Clone)]
pub struct ActionsModel {
    inner: Model<Actions>,
}

impl ActionsModel {
    pub fn init(cx: &mut WindowContext) -> Self {
        let inner = cx.new_model(|cx| Actions {
            global: Vec::new(),
            local: Vec::new(),
            combined: Vec::new(),
        });
        Self { inner }
    }
    pub fn update_global(&self, actions: Vec<Action>, cx: &mut WindowContext) {
        self.inner.update(cx, |model, cx| {
            model.global = actions;
            model.compute();
            cx.notify();
        });
    }
    pub fn update_local(&self, actions: Vec<Action>, cx: &mut WindowContext) {
        self.inner.update(cx, |model, cx| {
            model.local = actions;
            model.compute();
            cx.notify();
        });
    }
    pub fn get(&self, cx: &mut WindowContext) -> Vec<Action> {
        self.inner.read(cx).combined.clone()
    }
    pub fn check(&self, keystroke: &Keystroke, cx: &mut WindowContext) -> Option<Action> {
        let actions = &self.inner.read(cx).combined;
        for action in actions {
            if let Some(shortcut) = &action.shortcut {
                if shortcut.eq(keystroke) {
                    eprintln!("action: {:?}", action.label);
                    return Some(action.clone());
                }
            }
        }
        None
    }
}
