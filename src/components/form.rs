use std::collections::HashMap;

use gpui::*;

use crate::{
    components::shared::{Icon, Img},
    query::{TextEvent, TextInput},
    state::{Action, Actions, ActionsModel},
    theme::Theme,
};

pub struct Input {
    id: String,
    label: String,
    kind: InputKind,
    value: String,
    validate: Option<fn(&str) -> Option<&str>>,
    error: Option<String>,
    show_error: bool,
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
            .child(
                div()
                    .child(if self.focused {
                        self.input.view.clone().into_any_element()
                    } else {
                        if self.inner.value.is_empty() {
                            match self.inner.kind.clone() {
                                InputKind::PasswordField { placeholder }
                                | InputKind::TextArea { placeholder }
                                | InputKind::TextField { placeholder } => {
                                    placeholder.into_any_element()
                                }
                            }
                        } else {
                            match self.inner.kind.clone() {
                                InputKind::PasswordField { .. } => {
                                    "•".repeat(self.inner.value.clone().len())
                                }
                                _ => self.inner.value.clone(),
                            }
                            .into_any_element()
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
            InputKind::TextArea { placeholder } | InputKind::TextField { placeholder } => {
                self.input.set_masked(false, cx);
                self.input.set_placeholder(placeholder, cx);
            }
            InputKind::PasswordField { placeholder } => {
                self.input.set_masked(true, cx);
                self.input.set_placeholder(placeholder, cx);
            }
        };
        self.input.set_text(self.inner.value.clone(), cx);
    }
    pub fn on_blur(&mut self, _: &mut ViewContext<Self>) {
        self.inner.show_error = true;
        self.inner.error = self
            .inner
            .validate
            .map(|f| f(&self.inner.value))
            .flatten()
            .map(|s| s.to_string());
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
                match event {
                    TextEvent::Input { text } => {
                        input.inner.value = text.clone();
                        input.inner.error = input
                            .inner
                            .validate
                            .map(|f| f(text))
                            .flatten()
                            .map(|s| s.to_string());
                    }
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
                            input.focus_model.update(cx, |this, cx| {
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
                            input.focus_model.update(cx, |this, cx| {
                                *this += 1;
                                cx.notify();
                            })
                        }
                    }
                    _ => {}
                }
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

impl Input {
    pub fn new(
        id: impl ToString,
        label: impl ToString,
        kind: InputKind,
        value: impl ToString,
        validate: Option<fn(&str) -> Option<&str>>,
        _: &mut WindowContext,
    ) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            kind,
            value: value.to_string(),
            validate,
            error: None,
            show_error: false,
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum InputKind {
    TextField { placeholder: String },
    PasswordField { placeholder: String },
    TextArea { placeholder: String },
}

pub trait SubmitFn: Fn(HashMap<String, String>, &mut Actions, &mut WindowContext) -> () {
    fn clone_box<'a>(&self) -> Box<dyn 'a + SubmitFn>
    where
        Self: 'a;
}

impl<F> SubmitFn for F
where
    F: Fn(HashMap<String, String>, &mut Actions, &mut WindowContext) -> () + Clone,
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
                        let mut values = HashMap::<String, String>::new();
                        let mut error = false;
                        for input in inputs.clone() {
                            input.update(cx, |this, _| {
                                if this.inner.error.is_some() {
                                    error = true;
                                }
                                this.inner.show_error = true;
                                values.insert(this.inner.id.clone(), this.inner.value.clone());
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
