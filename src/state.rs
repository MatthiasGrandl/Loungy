/*
 *
 *  This source file is part of the Loungy open source project
 *
 *  Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 *  Licensed under MIT License
 *
 *  See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 *
 */

use gpui::*;
use log::debug;
use parking_lot::{Mutex, MutexGuard};
use serde::Deserialize;
use std::{
    ops::DerefMut,
    time::{Duration, Instant},
};

use crate::{
    // commands::root::list::RootListBuilder,
    components::{
        list::{Accessory, ItemBuilder, List, ListBuilder, ListItem},
        shared::{Icon, Img, ImgMask, ImgSize},
    },
    query::{TextEvent, TextInput, TextInputWeak},
    theme::{self, Theme},
    window::{Window, WindowStyle},
};

pub struct LazyMutex<T> {
    inner: Mutex<Option<T>>,
    init: fn() -> T,
}

impl<T> LazyMutex<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            inner: Mutex::new(None),
            init,
        }
    }

    pub fn lock(&self) -> impl DerefMut<Target = T> + '_ {
        MutexGuard::map(self.inner.lock(), |val| val.get_or_insert_with(self.init))
    }
}

#[derive(Clone, PartialEq)]
pub enum ToastState {
    Success {
        message: String,
        fade_in: Instant,
        fade_out: Option<Instant>,
    },
    Error {
        message: String,
        fade_in: Instant,
        fade_out: Option<Instant>,
    },
    Loading {
        message: String,
        fade_in: Instant,
        fade_out: Option<Instant>,
    },
    Idle,
}

impl ToastState {
    fn dot(color: Hsla) -> AnyElement {
        let size = Pixels(6.0);
        div()
            .size_6()
            .flex()
            .relative()
            .child(
                div()
                    .size(size)
                    .absolute()
                    .inset_0()
                    .m_auto()
                    .bg(color)
                    .rounded_full()
                    .with_animation(
                        "ping",
                        Animation::new(Duration::from_secs(2))
                            .with_easing(ease_in_out)
                            .repeat(),
                        {
                            move |div, delta| {
                                let delta = (delta - 0.75) * 4.0;
                                let mut color = color;
                                color.a = 1.0 - delta;
                                let size = Pixels(size.0 * delta * 2.0 + size.0);
                                div.bg(color).size(size)
                            }
                        },
                    ),
            )
            .child(
                div()
                    .size(size)
                    .absolute()
                    .inset_0()
                    .m_auto()
                    .bg(color)
                    .rounded_full(),
            )
            .into_any_element()
    }
    pub fn timeout(&mut self, duration: Duration, cx: &mut ViewContext<Self>) {
        cx.spawn(move |view, mut cx| async move {
            cx.background_executor().timer(duration).await;
            // cx.background_executor().timer(duration).await;
            let _ = view.update(&mut cx, |this, cx| {
                *this = ToastState::Idle;
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for ToastState {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<theme::Theme>();
        if let Some((el, bg, message, fade_in, fade_out)) = match self {
            ToastState::Success {
                message,
                fade_in,
                fade_out,
            } => Some((
                ToastState::dot(theme.green),
                theme.green,
                message,
                fade_in,
                fade_out,
            )),
            ToastState::Error {
                message,
                fade_in,
                fade_out,
            } => Some((
                ToastState::dot(theme.red),
                theme.red,
                message,
                fade_in,
                fade_out,
            )),
            ToastState::Loading {
                message,
                fade_in,
                fade_out,
            } => Some((
                Img::default()
                    .icon(Icon::Loader2)
                    .mask(ImgMask::None)
                    .size(ImgSize::SM)
                    .into_any_element(),
                theme.blue,
                message,
                fade_in,
                fade_out,
            )),
            ToastState::Idle => None,
        } {
            div()
                .absolute()
                .bottom_0()
                .h_full()
                .w_full()
                .p_2()
                .flex()
                .items_center()
                .child(div().child(el).mr_2().flex_shrink_0())
                .text_color(theme.text)
                .font_weight(FontWeight::MEDIUM)
                .child(message.to_string())
                .with_animation(
                    "toast-pulse",
                    Animation::new(Duration::from_secs(3))
                        .repeat()
                        .with_easing(bounce(ease_in_out)),
                    {
                        let fade_in = *fade_in;
                        let fade_out = *fade_out;
                        move |div, delta| {
                            let mut bg = bg;
                            let alpha = 0.1 + delta / 20.0;
                            let now = Instant::now();
                            let (left, alpha) = if fade_out.is_some() && now > fade_out.unwrap() {
                                let delta =
                                    now.duration_since(fade_out.unwrap()).as_secs_f32() / 0.3;
                                if delta < 1.0 {
                                    (ease_in_out(delta), alpha * ease_in_out(1.0 - delta))
                                } else {
                                    (1.0, 0.0)
                                }
                            } else {
                                let delta = now.duration_since(fade_in).as_secs_f32() / 0.3;
                                if delta < 1.0 {
                                    (-ease_in_out(1.0 - delta), alpha * ease_in_out(delta))
                                } else {
                                    (0.0, alpha)
                                }
                            };

                            bg.a = alpha;
                            div.bg(bg).left(relative(left))
                        }
                    },
                )
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }
}

#[derive(Clone)]
pub struct Toast {
    pub state: View<ToastState>,
}

impl Toast {
    pub fn init(cx: &mut WindowContext) -> Self {
        let state = cx.new_view(|_| ToastState::Idle);
        Self { state }
    }
    pub fn loading<C: VisualContext>(&mut self, message: impl ToString, cx: &mut C) {
        self.state.update(cx, |this, cx| {
            *this = ToastState::Loading {
                message: message.to_string(),
                fade_in: Instant::now(),
                fade_out: None,
            };
            cx.notify();
        });
    }
    pub fn success<C: VisualContext>(&mut self, message: impl ToString, cx: &mut C) {
        self.state.update(cx, |this, cx| {
            *this = ToastState::Success {
                message: message.to_string(),
                fade_in: Instant::now(),
                fade_out: Some(Instant::now() + Duration::from_secs(3)),
            };
            cx.notify();
        });
    }
    pub fn error<C: VisualContext>(&mut self, message: impl ToString, cx: &mut C) {
        self.state.update(cx, |this, cx| {
            *this = ToastState::Error {
                message: message.to_string(),
                fade_in: Instant::now(),
                fade_out: Some(Instant::now() + Duration::from_secs(4)),
            };
            cx.notify();
        });
    }
    /*
       TODO: This works in theory, but cx.hide() will hide the entire app so the toast won't show.
       I experimented with instead removing the main window with cx.remove_window() and restoring it on hotkey press, but then we lose all state.
       So right now I don't have a good solution. I am leaving this here for future reference investigation.
    */
    pub fn floating(&mut self, message: impl ToString, icon: Option<Icon>, cx: &mut WindowContext) {
        let bounds = cx.display().map(|d| d.bounds()).unwrap_or(Bounds {
            origin: Point::new(Pixels::from(0.0), Pixels::from(0.0)),
            size: Size {
                width: Pixels::from(1920.0),
                height: Pixels::from(1080.0),
            },
        });
        Window::close(cx);
        let _ = cx.open_window(
            WindowStyle::Toast {
                width: message.to_string().len() as u32 * 12,
                height: 50,
            }
            .options(bounds),
            |cx| {
                cx.spawn(|mut cx| async move {
                    cx.background_executor().timer(Duration::from_secs(2)).await;
                    //cx.background_executor().timer(Duration::from_secs(2)).await;
                    let _ = cx.update_window(cx.window_handle(), |_, cx| {
                        cx.remove_window();
                    });
                })
                .detach();
                cx.new_view(|_| PopupToast {
                    message: message.to_string(),
                    icon,
                })
            },
        );
    }
}
pub struct PopupToast {
    message: String,
    icon: Option<Icon>,
}

impl Render for PopupToast {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<theme::Theme>();

        let icon = if let Some(icon) = self.icon.clone() {
            svg()
                .path(icon.path())
                .text_color(theme.text)
                .size_5()
                .mr_2()
                .flex_shrink_0()
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .bg(theme.base)
            .text_color(theme.text)
            .font_weight(FontWeight::BOLD)
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(icon)
            .child(self.message.clone())
    }
}

#[derive(Clone)]
pub struct StateItem {
    pub id: String,
    pub query: TextInput,
    pub view: AnyView,
    pub actions: View<Actions>,
    pub workspace: bool,
}

pub struct StateViewContext {
    pub query: TextInputWeak,
    pub actions: ActionsModel,
    pub update_receiver: crossbeam_channel::Receiver<bool>,
}

impl StateItem {
    pub fn init(view: impl StateViewBuilder, workspace: bool, cx: &mut WindowContext) -> Self {
        let (s, r) = crossbeam_channel::unbounded::<bool>();
        let (actions_weak, actions) = ActionsModel::init(s, cx);
        let query = TextInput::new(cx);

        let actions_clone = actions_weak.clone();
        cx.subscribe(&query.view, move |_, event, cx| match event {
            TextEvent::Blur => {
                // if !actions_clone.inner.read(cx).show {
                //     Window::close(cx);
                // };
            }
            TextEvent::KeyDown(ev) => {
                let _ = actions_clone.inner.update(cx, |this, cx| {
                    if let Some(action) = this.check(&ev.keystroke, cx) {
                        if ev.is_held {
                            return;
                        }
                        (action.action)(this, cx);
                        return;
                    };
                    if !ev.is_held
                        && (Keystroke {
                            modifiers: Modifiers::default(),
                            key: "tab".to_string(),
                            ime_key: None,
                        })
                        .eq(&ev.keystroke)
                    {
                        this.dropdown_cycle(cx);
                    }
                });

                if ev.keystroke.key.as_str() == "escape" {
                    Window::close(cx);
                }
            }
            TextEvent::Back => {
                StateModel::update(|this, cx| this.pop(cx), cx);
            }
            _ => {}
        })
        .detach();
        let mut context = StateViewContext {
            query: query.downgrade(),
            actions: actions_weak,
            update_receiver: r,
        };
        let id = view.command();
        let view = view.build(&mut context, cx);
        Self {
            id,
            query,
            view,
            actions,
            workspace,
        }
    }
}

pub trait StateViewBuilder: CommandTrait + Clone {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView;
}

pub trait CommandTrait {
    fn command(&self) -> String;
}

#[macro_export]
macro_rules! command {
    ($ty:ty) => {
        impl CommandTrait for $ty {
            fn command(&self) -> String {
                module_path!().to_string()
            }
        }
    };
}

pub struct State {
    pub stack: Vec<StateItem>,
}

#[derive(Clone)]
pub struct StateModel {
    pub inner: Model<State>,
}

impl StateModel {
    pub fn init(cx: &mut WindowContext) -> Self {
        let this = Self {
            inner: cx.new_model(|_| State { stack: vec![] }),
        };
        // this.push(RootListBuilder {}, cx);
        cx.set_global(this.clone());

        this
    }
    pub fn update(f: impl FnOnce(&mut Self, &mut WindowContext), cx: &mut WindowContext) {
        if !cx.has_global::<Self>() {
            log::error!("StateModel not found");
            return;
        }
        cx.update_global::<Self, _>(|this, cx| {
            f(this, cx);
        });
    }
    pub fn update_async(
        f: impl FnOnce(&mut Self, &mut WindowContext),
        cx: &mut AsyncWindowContext,
    ) {
        let _ = cx.update_global::<Self, _>(|this, cx| {
            f(this, cx);
        });
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
        let item = StateItem::init(view, true, cx);
        self.inner.update(cx, |model, cx| {
            model.stack.push(item);
            cx.notify();
        });
    }

    pub fn push_item(&self, item: StateItem, cx: &mut WindowContext) {
        self.inner.update(cx, |model, cx| {
            model.stack.push(item);
            cx.notify();
        });
    }
    pub fn replace(&self, view: impl StateViewBuilder, cx: &mut WindowContext) {
        self.pop(cx);
        self.push(view, cx);
    }
    pub fn reset(&self, cx: &mut WindowContext) {
        self.inner
            .update(cx, |model, _| {
                model.stack.truncate(1);
                model.stack.get(0).and_then(|i| Some(i.query.downgrade()))
            })
            .and_then(|q| Some(q.set_text("", cx)));
    }
}

impl Global for StateModel {}

// Actions

pub trait CloneableFn: Fn(&mut Actions, &mut WindowContext) {
    fn clone_box<'a>(&self) -> Box<dyn 'a + CloneableFn>
    where
        Self: 'a;
}

impl<F> CloneableFn for F
where
    F: Fn(&mut Actions, &mut WindowContext) + Clone,
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

// implement Default for CloneableFn
impl Default for Box<dyn CloneableFn> {
    fn default() -> Self {
        Box::new(|_, _| {})
    }
}

#[derive(Clone, IntoElement, Deserialize)]
pub struct Shortcut {
    inner: Keystroke,
}

impl From<&Keystroke> for Shortcut {
    fn from(keystroke: &Keystroke) -> Self {
        Self {
            inner: keystroke.clone(),
        }
    }
}

impl Shortcut {
    pub fn new(key: impl ToString) -> Self {
        Self {
            inner: Keystroke {
                modifiers: Modifiers::default(),
                key: key.to_string(),
                ime_key: None,
            },
        }
    }
    pub fn cmd(mut self) -> Self {
        #[cfg(target_os = "macos")]
        {
            self.inner.modifiers.platform = true;
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.inner.modifiers.control = true;
        }
        self
    }
    pub fn shift(mut self) -> Self {
        self.inner.modifiers.shift = true;
        self
    }
    pub fn alt(mut self) -> Self {
        self.inner.modifiers.alt = true;
        self
    }
    pub fn ctrl(mut self) -> Self {
        #[cfg(target_os = "macos")]
        {
            self.inner.modifiers.control = true;
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.inner.modifiers.platform = true;
        }
        self
    }
    pub fn get(&self) -> Keystroke {
        self.inner.clone()
    }
}

fn key_icon(el: Div, icon: Icon) -> Div {
    el.child(
        div()
            .child(
                Img::default()
                    .icon(icon)
                    .size(ImgSize::SM)
                    .mask(ImgMask::Rounded),
            )
            .ml_0p5(),
    )
}

fn key_string(el: Div, theme: &Theme, string: impl ToString) -> Div {
    el.child(
        div()
            .size_5()
            .p_1()
            .rounded_md()
            .bg(theme.surface0)
            .text_color(theme.text)
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .justify_center()
            .child(string.to_string())
            .ml_0p5(),
    )
}

impl RenderOnce for Shortcut {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<theme::Theme>();
        let mut el = div().flex().items_center();
        let shortcut = self.inner;
        if shortcut.modifiers.control {
            el = key_icon(el, Icon::ChevronUp);
        }
        if shortcut.modifiers.alt {
            el = key_icon(el, Icon::Option);
        }
        if shortcut.modifiers.shift {
            el = key_icon(el, Icon::ArrowBigUp);
        }
        if shortcut.modifiers.platform {
            el = key_icon(el, Icon::Command);
        }
        match shortcut.key.as_str() {
            "enter" => {
                el = key_icon(el, Icon::CornerDownLeft);
            }
            "backspace" => {
                el = key_icon(el, Icon::Delete);
            }
            "delete" => {
                el = key_icon(el, Icon::Delete);
            }
            "escape" => {
                el = key_icon(el, Icon::ArrowUpRightFromSquare);
            }
            "tab" => {
                el = key_icon(el, Icon::ArrowRightToLine);
            }
            "space" => {
                el = key_icon(el, Icon::Space);
            }
            "up" => {
                el = key_icon(el, Icon::ArrowUp);
            }
            "down" => {
                el = key_icon(el, Icon::ArrowDown);
            }
            "left" => {
                el = key_icon(el, Icon::ArrowLeft);
            }
            "right" => {
                el = key_icon(el, Icon::ArrowRight);
            }
            "comma" => {
                el = key_string(el, theme, ",");
            }
            "dot" => {
                el = key_string(el, theme, ".");
            }
            "questionmark" => {
                el = key_string(el, theme, "?");
            }
            "exclamationmark" => {
                el = key_string(el, theme, "!");
            }
            "slash" => {
                el = key_string(el, theme, "/");
            }
            "backslash" => {
                el = key_string(el, theme, "\\");
            }
            _ => {
                el = key_string(
                    el,
                    theme,
                    shortcut.ime_key.unwrap_or(shortcut.key).to_uppercase(),
                );
            }
        }
        el
    }
}

#[derive(Clone, IntoElement)]
pub struct Action {
    pub label: String,
    pub shortcut: Option<Shortcut>,
    pub image: Img,
    pub action: Box<dyn CloneableFn>,
    pub hide: bool,
}

impl RenderOnce for Action {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        let shortcut = if let Some(shortcut) = self.shortcut {
            div().child(shortcut)
        } else {
            div()
        };
        div()
            .ml_auto()
            .child(div().child(self.label).mr_2())
            .flex()
            .items_center()
            .justify_between()
            .child(shortcut)
    }
}

impl Action {
    pub fn new(
        image: Img,
        label: impl ToString,
        shortcut: Option<Shortcut>,
        action: impl CloneableFn + 'static,
        hide: bool,
    ) -> Self {
        Self {
            label: label.to_string(),
            shortcut,
            action: Box::new(action),
            image,
            hide,
        }
    }
}

#[derive(Clone)]
pub struct Dropdown {
    value: String,
    items: Vec<(String, String)>,
}

impl Render for Dropdown {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<theme::Theme>();
        if self.items.is_empty() {
            return div();
        }

        let label = self
            .items
            .iter()
            .find(|item| item.0.eq(&self.value))
            .unwrap()
            .1
            .clone();
        div()
            .px_2()
            .py_0p5()
            .rounded_lg()
            .bg(theme.mantle)
            .ml_auto()
            .flex()
            .items_center()
            .justify_between()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.subtext0)
            .border_1()
            .border_color(theme.crust)
            .child(div().child(label))
    }
}

#[derive(Clone)]
pub struct Actions {
    global: Model<Vec<Action>>,
    local: Model<Vec<Action>>,
    pub active: Option<StateItem>,
    meta: Option<AnyModel>,
    show: bool,
    query: Option<TextInput>,
    list: Option<View<List>>,
    update_sender: crossbeam_channel::Sender<bool>,
    pub toast: Toast,
    pub dropdown: View<Dropdown>,
}

impl Actions {
    fn new(update_sender: crossbeam_channel::Sender<bool>, cx: &mut WindowContext) -> Self {
        Self {
            global: cx.new_model(|_| Vec::new()),
            local: cx.new_model(|_| Vec::new()),
            active: None,
            meta: None,
            show: false,
            query: None,
            list: None,
            toast: Toast::init(cx),
            dropdown: cx.new_view(|_| Dropdown {
                value: "".to_string(),
                items: vec![],
            }),
            update_sender,
        }
    }
    pub fn default(cx: &mut WindowContext) -> Self {
        let (s, _) = crossbeam_channel::unbounded::<bool>();
        Self::new(s, cx)
    }
    fn combined(&self, cx: &WindowContext) -> Vec<Action> {
        let mut combined = self.local.read(cx).clone();
        combined.append(&mut self.global.read(cx).clone());
        if let Some(action) = combined.get_mut(0) {
            let key = "enter";
            action.shortcut = Some(Shortcut::new(key));
            combined.push(Action::new(
                Img::default().icon(Icon::BookOpen),
                "Actions",
                Some(Shortcut::new("k").cmd()),
                |this, cx| {
                    this.update_list(cx);
                    this.show = !this.show;
                },
                true,
            ))
        }
        combined
    }
    fn update_list(&self, cx: &mut WindowContext) {
        if let Some(list) = &self.list {
            list.update(cx, |this, cx| {
                this.reset_selection(cx);
                this.update(true, cx);
            });
        }
    }
    fn popup(&mut self, cx: &mut ViewContext<Self>) -> Div {
        if !self.show {
            return div();
        }
        let theme = cx.global::<theme::Theme>();
        let query = self.query.clone().unwrap();
        let list = self.list.clone().unwrap();
        let el_height = 42.0;
        let count = list.read(cx).items.read(cx).len();
        let height = Pixels(if count == 0 {
            0.0
        } else if count > 4 {
            4.0 * el_height + 20.0
        } else {
            count as f32 * el_height + 20.0
        });

        div()
            .absolute()
            .bottom_10()
            .right_0()
            .w_80()
            .bg(theme.base)
            .rounded_xl()
            .border_1()
            .border_color(theme.crust)
            .shadow_lg()
            .flex()
            .flex_col()
            .child(if count > 0 {
                div().h(height).p_2().child(list)
            } else {
                div()
                    .child("No results")
                    .px_2()
                    .py_8()
                    .flex()
                    .items_center()
                    .justify_center()
            })
            .child(
                div()
                    .flex_shrink_0()
                    .child(query)
                    .mt_auto()
                    .px_2()
                    .border_t_1()
                    .border_color(theme.mantle)
                    .text_sm(),
            )
    }
    fn check(&self, keystroke: &Keystroke, cx: &WindowContext) -> Option<Action> {
        let actions = self.combined(cx);
        for action in actions {
            if let Some(shortcut) = &action.shortcut {
                if shortcut.inner.eq(keystroke) {
                    return Some(action.clone());
                }
            }
        }
        None
    }
    pub fn update(&self) {
        let _ = self.update_sender.send(true);
    }
    pub fn set_dropdown_value(&mut self, value: impl ToString, cx: &mut WindowContext) {
        self.dropdown.update(cx, |this, cx| {
            let value = value.to_string();
            if !value.is_empty() {
                if this.items.iter().any(|item| item.0.eq(&value)) {
                    this.value = value;
                    cx.notify();
                };
            } else {
                this.value = value;
                cx.notify();
            }
        });
        self.update()
    }
    pub fn dropdown_cycle(&mut self, cx: &mut WindowContext) {
        self.dropdown.update(cx, |this, cx| {
            if this.items.is_empty() {
                return;
            }
            let index = this
                .items
                .iter()
                .position(|item| item.0.eq(&this.value))
                .unwrap_or(0);
            let next = (index + 1) % this.items.len();
            this.value = this.items[next].0.clone();
            cx.notify();
        });
        self.update()
    }
    pub fn has_focus(&self, cx: &WindowContext) -> bool {
        self.query.as_ref().unwrap().downgrade().has_focus(cx)
    }
    pub fn get_meta_model<V: Clone + 'static>(&self) -> Option<Model<V>> {
        self.meta.clone().and_then(|m| m.downcast::<V>().ok())
    }
    pub fn get_meta<V: Clone + 'static>(&self, cx: &AppContext) -> Option<V> {
        self.get_meta_model().map(|v| v.read(cx)).cloned()
    }
}

impl Render for Actions {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let combined = self.combined(cx).clone();
        let theme = cx.global::<theme::Theme>();
        if let Some(action) = combined.first() {
            let open = combined.last().unwrap().clone();
            div()
                .ml_auto()
                .flex()
                .items_center()
                .font_weight(FontWeight::SEMIBOLD)
                .child(div().child(action.clone()).text_color(theme.text))
                .child(div().h_2_3().w(Pixels(2.0)).bg(theme.surface0).mx_2())
                .child(open)
                .child(self.popup(cx))
        } else {
            div().child(" ")
        }
    }
}

#[derive(Clone)]
pub struct ActionsModel {
    pub inner: WeakView<Actions>,
}

impl ActionsModel {
    pub fn init(
        update_sender: crossbeam_channel::Sender<bool>,
        cx: &mut WindowContext,
    ) -> (Self, View<Actions>) {
        let inner = cx.new_view(|cx| {
            #[cfg(debug_assertions)]
            cx.on_release(|_, _, _| debug!("ActionsModel released"))
                .detach();
            Actions::new(update_sender, cx)
        });

        let model = Self {
            inner: inner.downgrade(),
        };
        inner.update(cx, |this, cx| {
            let (_s, r) = crossbeam_channel::unbounded::<bool>();
            let query = TextInput::new(cx);
            let mut context = StateViewContext {
                query: query.downgrade(),
                actions: model.clone(),
                update_receiver: r,
            };

            context.query.set_placeholder("Search for actions...", cx);

            let actions = this.clone();
            let list = ListBuilder::new().disable_action_updates().build(
                move |_, _, cx| {
                    let actions = actions.combined(cx);
                    Ok(Some(
                        actions
                            .into_iter()
                            .filter_map(|item| {
                                if item.hide {
                                    return None;
                                }
                                let action = item.clone();
                                let accessories = if let Some(shortcut) = item.shortcut {
                                    vec![Accessory::shortcut(shortcut)]
                                } else {
                                    vec![]
                                };
                                Some(
                                    ItemBuilder::new(
                                        item.label.clone(),
                                        ListItem::new(
                                            Some(action.image.clone()),
                                            item.label.clone(),
                                            None,
                                            accessories,
                                        ),
                                    )
                                    .keywords(vec![item.label.clone()])
                                    .actions(vec![action])
                                    .build(),
                                )
                            })
                            .collect(),
                    ))
                },
                &mut context,
                cx,
            );
            let list_clone = list.downgrade();
            this.list = Some(list);
            cx.subscribe(&query.view, move |this, _, event, cx| {
                match event {
                    TextEvent::Blur | TextEvent::Back => {
                        this.show = false;
                        cx.notify();
                    }
                    TextEvent::KeyDown(ev) => {
                        let key = "enter";
                        if Shortcut::new(key).get().eq(&ev.keystroke) {
                            this.show = false;
                            cx.notify();
                            let _ = list_clone.update(cx, |this2, cx| {
                                if let Some(action) = this2.default_action(cx) {
                                    (action.action)(this, cx);
                                }
                            });
                            return;
                        }
                        if let Some(action) = this.check(&ev.keystroke, cx) {
                            if ev.is_held {
                                return;
                            }
                            (action.action)(this, cx);
                            return;
                        }
                        if ev.keystroke.key.as_str() == "escape" {
                            this.show = false;
                            cx.notify();
                        }
                    }
                    _ => {}
                }
                cx.notify();
            })
            .detach();

            this.query = Some(query);

            cx.notify();
        });
        (model, inner)
    }
    pub fn update_global(&self, actions: Vec<Action>, cx: &mut WindowContext) {
        let _ = self.inner.update(cx, |model, cx| {
            model.global.update(cx, |this, cx| {
                *this = actions;
                cx.notify();
            });
            model.update_list(cx);
        });
    }
    pub fn update_local(
        &self,
        actions: Vec<Action>,
        item: Option<StateItem>,
        meta: Option<AnyModel>,
        cx: &mut WindowContext,
    ) {
        let _ = self.inner.update(cx, |model, cx| {
            model.active = item;
            model.meta = meta;
            model.local.update(cx, |this, cx| {
                *this = actions;
                cx.notify();
            });
            model.update_list(cx);
        });
    }
    pub fn clear_local(&self, cx: &mut WindowContext) {
        let _ = self.inner.update(cx, |model, cx| {
            model.active = None;
            model.local.update(cx, |this, cx| {
                this.clear();
                cx.notify();
            });
            model.update_list(cx);
        });
    }
    pub fn get_dropdown_value(&self, cx: &WindowContext) -> String {
        self.inner
            .upgrade()
            .map(|this| this.read(cx).dropdown.read(cx).value.clone())
            .unwrap_or_default()
    }
    pub fn set_dropdown(
        &mut self,
        value: impl ToString,
        items: Vec<(impl ToString, impl ToString)>,
        cx: &mut WindowContext,
    ) {
        let _ = self.inner.update(cx, |model, cx| {
            model.dropdown.update(cx, |this, cx| {
                this.items = items
                    .into_iter()
                    .map(|(value, label)| (value.to_string(), label.to_string()))
                    .collect();
                cx.notify();
            });
            model.set_dropdown_value(value, cx);
            cx.notify();
        });
    }
}
