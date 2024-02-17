use std::{any::Any, collections::HashMap};

use gpui::*;

use crate::{
    components::shared::{Icon, Img},
    query::{TextEvent, TextInput},
    state::{Action, Actions, ActionsModel, Shortcut},
    theme::Theme,
};

#[derive(Clone)]
pub struct Input {
    id: String,
    label: String,
    kind: InputKind,
    error: Option<String>,
    show_error: bool,
}

impl Input {
    pub fn new(
        id: impl ToString,
        label: impl ToString,
        kind: InputKind,
        _: &mut WindowContext,
    ) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            kind,
            error: None,
            show_error: false,
        }
    }
    pub fn validate(&mut self) {
        self.error = match &self.kind {
            InputKind::TextField {
                value, validate, ..
            } => validate.map(|f| f(&value)).flatten().map(|s| s.to_string()),
            _ => None,
        }
    }
    pub fn value<V: Clone + 'static>(&self) -> V {
        let value: Box<dyn Any> = match self.kind.clone() {
            InputKind::TextField { value, .. } => Box::new(value),
            InputKind::Shortcut { value } => Box::new(value),
            _ => Box::new("".to_string()),
        };
        value.downcast_ref::<V>().unwrap().clone()
    }
}

pub struct InputView {
    inner: Input,
    input: TextInput,
    focused: bool,
    index: usize,
    focus_model: Model<usize>,
}

impl Render for InputView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        //cx.focus(&self.focus_handle);
        let theme = cx.global::<Theme>();
        let fm = self.focus_model.clone();
        let index = self.index.clone();

        div()
            .flex()
            .items_center()
            .text_sm()
            .child(
                div()
                    .w_1_4()
                    .pr_2()
                    .child(self.inner.label.clone())
                    .font_weight(FontWeight::SEMIBOLD)
                    .flex()
                    .justify_end(),
            )
            .text_color(theme.subtext0)
            .relative()
            .child(
                div()
                    .child(if self.focused {
                        match self.inner.kind.clone() {
                            InputKind::TextField { .. } => {
                                self.input.view.clone().into_any_element()
                            }
                            InputKind::Shortcut { .. } => "Recording...".into_any_element(),
                        }
                    } else {
                        match self.inner.kind.clone() {
                            InputKind::TextField {
                                placeholder,
                                value,
                                password,
                                ..
                            } => {
                                if value.is_empty() {
                                    placeholder.into_any_element()
                                } else {
                                    if password {
                                        "•".repeat(value.len()).into_any_element()
                                    } else {
                                        value.into_any_element()
                                    }
                                }
                            }
                            InputKind::Shortcut { value } => {
                                if let Some(shortcut) = value {
                                    div()
                                        .child(" ")
                                        .child(
                                            div()
                                                .absolute()
                                                .inset_0()
                                                .flex()
                                                .items_center()
                                                .child(shortcut.into_any_element()),
                                        )
                                        .into_any_element()
                                } else {
                                    "Record Hotkey".into_any_element()
                                }
                            }
                        }
                    })
                    .w_1_2()
                    .p_2()
                    .border_1()
                    .rounded_lg()
                    .border_color(if self.focused {
                        theme.lavender
                    } else if self.inner.show_error && self.inner.error.is_some() {
                        theme.red
                    } else {
                        theme.surface0
                    }),
            )
            .child(div().w_1_4().pl_2().text_color(theme.red).child(
                if let Some(error) = self.inner.error.clone() {
                    if self.inner.show_error {
                        error.into_any_element()
                    } else {
                        div().into_any_element()
                    }
                } else {
                    div().into_any_element()
                },
            ))
            .on_mouse_down(MouseButton::Left, move |_, cx| {
                fm.update(cx, |this, cx| {
                    *this = index;
                    cx.notify();
                })
            })
    }
}

impl InputView {
    pub fn on_focus(&mut self, cx: &mut ViewContext<Self>) {
        //
        match self.inner.kind.clone() {
            InputKind::TextField {
                placeholder,
                value,
                password,
                ..
            } => {
                self.input.set_masked(password, cx);
                self.input.set_placeholder(placeholder, cx);
                self.input.set_text(value, cx);
            }
            InputKind::Shortcut { .. } => self.input.set_placeholder("Record hotkey", cx),
        };
    }
    pub fn on_blur(&mut self, _: &mut ViewContext<Self>) {
        self.inner.show_error = true;
        self.inner.validate();
    }
    pub fn on_query(&mut self, event: &TextEvent, cx: &mut ViewContext<Self>) {
        match self.inner.kind.clone() {
            InputKind::TextField {
                validate,
                placeholder,
                password,
                ..
            } => match event {
                TextEvent::Input { text } => {
                    self.inner.kind = InputKind::TextField {
                        value: text.clone(),
                        validate,
                        placeholder,
                        password,
                    };
                    self.inner.validate();
                }
                TextEvent::KeyDown(e) => {}
                _ => {}
            },
            InputKind::Shortcut { value } => match event {
                TextEvent::KeyDown(e) => {
                    self.input.set_text("", cx);

                    let mods = e.keystroke.modifiers;
                    if mods.shift || mods.control || mods.alt || mods.command {
                        self.inner.kind = InputKind::Shortcut {
                            value: Some(Shortcut::new(e.keystroke.clone())),
                        };
                        self.focus_model.update(cx, |this, cx| {
                            *this += 1;
                            cx.notify();
                        });
                    }
                }
                _ => {}
            },
        }
        match event {
            TextEvent::KeyDown(e) => {
                if (Keystroke {
                    key: "tab".to_string(),
                    modifiers: Modifiers {
                        shift: true,
                        ..Modifiers::default()
                    },
                    ime_key: None,
                })
                .eq(&e.keystroke)
                {
                    self.focus_model.update(cx, |this, cx| {
                        if this > &mut 0 {
                            *this -= 1;
                            cx.notify();
                        }
                    })
                    //
                } else if (Keystroke {
                    key: "tab".to_string(),
                    modifiers: Modifiers::default(),
                    ime_key: None,
                })
                .eq(&e.keystroke)
                {
                    self.focus_model.update(cx, |this, cx| {
                        *this += 1;
                        cx.notify();
                    })
                }
            }
            _ => {}
        }
    }
    pub fn new(
        input: Input,
        query: TextInput,
        index: usize,
        focus_model: Model<usize>,
        cx: &mut WindowContext,
    ) -> View<Self> {
        cx.new_view(|cx| {
            cx.observe(&focus_model, move |input: &mut InputView, focused, cx| {
                let old = input.focused;
                let new = index.eq(focused.read(cx));
                if old == new {
                    return;
                }
                input.focused = new;
                if input.focused {
                    input.on_focus(cx);
                } else {
                    input.on_blur(cx);
                }
                cx.notify();
            })
            .detach();
            cx.subscribe(&query.view, |input: &mut Self, _, event: &TextEvent, cx| {
                if !input.focused {
                    return;
                }
                input.on_query(event, cx);
                //
            })
            .detach();
            Self {
                inner: input,
                input: query,
                focused: false,
                index,
                focus_model: focus_model.clone(),
            }
        })
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum InputKind {
    TextField {
        placeholder: String,
        value: String,
        password: bool,
        validate: Option<fn(&str) -> Option<&str>>,
    },
    Shortcut {
        value: Option<Shortcut>,
    },
}

pub trait SubmitFn: Fn(HashMap<String, Input>, &mut Actions, &mut WindowContext) -> () {
    fn clone_box<'a>(&self) -> Box<dyn 'a + SubmitFn>
    where
        Self: 'a;
}

impl<F> SubmitFn for F
where
    F: Fn(HashMap<String, Input>, &mut Actions, &mut WindowContext) -> () + Clone,
{
    fn clone_box<'a>(&self) -> Box<dyn 'a + SubmitFn>
    where
        Self: 'a,
    {
        Box::new(self.clone())
    }
}

impl<'a> Clone for Box<dyn 'a + SubmitFn> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

pub struct Form {
    list: ListState,
}

impl Form {
    pub fn new(
        inputs: Vec<Input>,
        submit: impl SubmitFn + 'static,
        query: &TextInput,
        actions: &ActionsModel,
        cx: &mut WindowContext,
    ) -> View<Self> {
        let focus_model: Model<usize> = cx.new_model(|_| 0);
        let inputs: Vec<View<InputView>> = inputs
            .into_iter()
            .enumerate()
            .map(|(i, input)| InputView::new(input, query.clone(), i, focus_model.clone(), cx))
            .collect();
        focus_model.update(cx, |_, cx| {
            cx.notify();
        });

        actions.update_local(
            vec![Action::new(
                Img::list_icon(Icon::PlusSquare, None),
                "Submit",
                None,
                {
                    let inputs = inputs.clone();
                    let actions = actions.inner.read(cx).clone();
                    let submit = submit.clone_box();
                    move |_, cx| {
                        let mut values = HashMap::<String, Input>::new();
                        let mut error = false;
                        for input in inputs.clone() {
                            input.update(cx, |this, _| {
                                if this.inner.error.is_some() {
                                    error = true;
                                }
                                this.inner.show_error = true;
                                values.insert(this.inner.id.clone(), this.inner.clone());
                            })
                        }
                        if error {
                            return;
                        }
                        submit(values, &mut actions.clone(), cx);
                    }
                },
                false,
            )],
            cx,
        );
        cx.new_view(|_| Self {
            list: ListState::new(
                inputs.len(),
                ListAlignment::Top,
                Pixels(100.0),
                move |i, _| div().child(inputs[i].clone()).py_2().into_any_element(),
            ),
        })
    }
}

impl Render for Form {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .p_4()
            .size_full()
            .child(list(self.list.clone()).size_full())
    }
}
