use ::simple_easing::linear;
use async_std::task::sleep;
use gpui::*;
use serde::Deserialize;
use std::{
    sync::mpsc::{channel, Receiver, Sender},
    time::{self, Duration},
};

use crate::{
    commands::root::list::RootListBuilder,
    components::list::{Accessory, Item, List, ListItem},
    components::shared::{Icon, Img, ImgMask, ImgSize, ImgSource},
    query::{TextEvent, TextInput},
    theme::{self, Theme},
    window::{Window, WindowStyle, WIDTH},
};

pub struct Loading {
    pub inner: bool,
    left: f32,
    width: f32,
}

impl Loading {
    fn init(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(|cx| {
            cx.spawn(|view, mut cx| async move {
                let easing: fn(f32) -> f32 = linear;
                let ts = time::Instant::now();
                let w = WIDTH as f32;
                let w_start = w * 0.4;
                let w_stop = w * 0.5;
                loop {
                    if view.upgrade().is_none() {
                        break;
                    }
                    let i = (ts.elapsed().as_millis() as f32 % 1000.0) / 1000.0;
                    let (left, width) = if i > 0.4 {
                        let i = (i - 0.4) / 0.6;
                        let e = easing(i);
                        let left = e * w;
                        let width = e * (w_stop - w_start) + w_start;
                        (left, width)
                    } else {
                        let i = i / 0.4;
                        (0.0, easing(i) * w_start)
                    };
                    let _ = cx.update(|cx| {
                        view.update(cx, |this: &mut Loading, cx| {
                            this.left = left;
                            this.width = width;
                            cx.notify();
                        })
                    });
                    sleep(Duration::from_millis(1000 / 120)).await;
                    // cx.background_executor()
                    //     .timer(Duration::from_millis(1000 / 120))
                    //     .await;
                }
            })
            .detach();

            Self {
                inner: false,
                left: 0.0,
                width: 0.0,
            }
        })
    }
}

impl Render for Loading {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<theme::Theme>();
        let mut bg = theme.lavender;
        bg.fade_out(0.2);
        let el = div().w_full().h_px().bg(theme.mantle).relative();
        if self.inner {
            el.child(
                div()
                    .absolute()
                    .h_full()
                    .top_0()
                    .bottom_0()
                    .bg(bg)
                    .left(Pixels(self.left))
                    .w(Pixels(self.width)),
            )
        } else {
            el
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum ToastState {
    Success(String),
    Error(String),
    Loading(String),
    Idle,
}

impl ToastState {
    fn dot(color: Hsla) -> AnyElement {
        div()
            .size_6()
            .flex()
            .items_center()
            .justify_center()
            .child(div().size_1p5().bg(color).rounded_full())
            .into_any_element()
    }
    pub fn timeout(&mut self, duration: Duration, cx: &mut ViewContext<Self>) {
        cx.spawn(move |view, mut cx| async move {
            sleep(duration).await;
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
        if let Some((el, mut bg, message)) = match self {
            ToastState::Success(message) => {
                Some((ToastState::dot(theme.green), theme.green, message))
            }
            ToastState::Error(message) => Some((ToastState::dot(theme.red), theme.red, message)),
            ToastState::Loading(message) => Some((
                Img::accessory_icon(Icon::Loader2, None).into_any_element(),
                theme.blue,
                message,
            )),
            ToastState::Idle => None,
        } {
            bg.fade_out(0.95);

            div()
                .absolute()
                .inset_0()
                .bottom_px()
                .p_2()
                .bg(bg)
                .flex()
                .items_center()
                .child(div().child(el).mr_2().flex_shrink_0())
                .text_color(theme.text)
                .font_weight(FontWeight::MEDIUM)
                .child(message.to_string())
        } else {
            div()
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
            *this = ToastState::Loading(message.to_string());
            cx.notify();
        });
    }
    pub fn success<C: VisualContext>(&mut self, message: impl ToString, cx: &mut C) {
        self.state.update(cx, |this, cx| {
            *this = ToastState::Success(message.to_string());
            cx.notify();
            this.timeout(Duration::from_secs(2), cx);
        });
    }
    pub fn error<C: VisualContext>(&mut self, message: impl ToString, cx: &mut C) {
        self.state.update(cx, |this, cx| {
            *this = ToastState::Error(message.to_string());
            cx.notify();
            this.timeout(Duration::from_secs(2), cx);
        });
    }
    /*
       TODO: This works in theory, but cx.hide() will hide the entire app so the toast won't show.
       I experimented with instead removing the main window with cx.remove_window() and restoring it on hotkey press, but then we lose all state.
       So right now I have a good solution. I am leaving this here for future reference investigation.
    */
    pub fn floating(&mut self, message: impl ToString, icon: Option<Icon>, cx: &mut WindowContext) {
        let bounds = cx.display().map(|d| d.bounds()).unwrap_or(Bounds {
            origin: Point::new(GlobalPixels::from(0.0), GlobalPixels::from(0.0)),
            size: Size {
                width: GlobalPixels::from(1920.0),
                height: GlobalPixels::from(1080.0),
            },
        });
        Window::close(cx);
        cx.open_window(
            WindowStyle::Toast {
                width: message.to_string().len() as f64 * 12.0,
                height: 50.0,
            }
            .options(bounds),
            |cx| {
                cx.spawn(|mut cx| async move {
                    sleep(Duration::from_secs(2)).await;
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
    pub query: TextInput,
    pub view: AnyView,
    pub actions: ActionsModel,
    pub workspace: bool,
}

impl StateItem {
    pub fn init(view: impl StateViewBuilder, workspace: bool, cx: &mut WindowContext) -> Self {
        let (s, r) = channel::<bool>();
        let actions = ActionsModel::init(s, cx);
        let query = TextInput::new(cx);

        let actions_clone = actions.clone();
        cx.subscribe(&query.view, move |_, event, cx| match event {
            TextEvent::Blur => {
                // if !actions_clone.inner.read(cx).show {
                //     Window::close(cx);
                // };
            }
            TextEvent::KeyDown(ev) => {
                actions_clone.inner.update(cx, |this, cx| {
                    if let Some(action) = this.check(&ev.keystroke, cx) {
                        if ev.is_held {
                            return;
                        }
                        (action.action)(this, cx);
                        return;
                    };
                });

                match ev.keystroke.key.as_str() {
                    "escape" => {
                        Window::close(cx);
                    }
                    _ => {}
                }
            }
            TextEvent::Back => {
                StateModel::update(|this, cx| this.pop(cx), cx);
            }
            _ => {}
        })
        .detach();
        let view = view.build(&query, &actions, r, cx);
        Self {
            query,
            view,
            actions,
            workspace,
        }
    }
}

pub trait StateViewBuilder: Clone {
    fn build(
        &self,
        query: &TextInput,
        actions: &ActionsModel,
        update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView;
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
        this.push(RootListBuilder {}, cx);
        cx.set_global(this.clone());
        this
    }
    pub fn update(f: impl FnOnce(&mut Self, &mut WindowContext), cx: &mut WindowContext) {
        cx.update_global::<Self, _>(|mut this, cx| {
            f(&mut this, cx);
        });
    }
    pub fn update_async(
        f: impl FnOnce(&mut Self, &mut WindowContext),
        cx: &mut AsyncWindowContext,
    ) {
        cx.update_global::<Self, _>(|mut this, cx| {
            f(&mut this, cx);
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
        self.inner.update(cx, |model, cx| {
            model.stack.truncate(1);
            //model.stack[0].query.set_text("", cx);
        });
    }
}

impl Global for StateModel {}

// Actions

pub trait CloneableFn: Fn(&mut Actions, &mut WindowContext) -> () {
    fn clone_box<'a>(&self) -> Box<dyn 'a + CloneableFn>
    where
        Self: 'a;
}

impl<F> CloneableFn for F
where
    F: Fn(&mut Actions, &mut WindowContext) -> () + Clone,
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

#[derive(Clone, IntoElement, Deserialize)]
pub struct Shortcut {
    pub inner: Keystroke,
}

impl Shortcut {
    pub fn simple(key: impl ToString) -> Self {
        Self {
            inner: Keystroke {
                modifiers: Modifiers::default(),
                key: key.to_string(),
                ime_key: None,
            },
        }
    }
    pub fn cmd(key: impl ToString) -> Self {
        Self {
            inner: Keystroke {
                modifiers: Modifiers {
                    #[cfg(target_os = "macos")]
                    command: true,
                    #[cfg(not(target_os = "macos"))]
                    control: true,
                    ..Modifiers::default()
                },
                key: key.to_string(),
                ime_key: None,
            },
        }
    }
    pub fn new(keystroke: Keystroke) -> Self {
        Self { inner: keystroke }
    }
}

fn key_icon(el: Div, icon: Icon) -> Div {
    el.child(
        div()
            .child(Img::new(
                ImgSource::Icon { icon, color: None },
                ImgMask::Rounded,
                ImgSize::SM,
            ))
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
        if shortcut.modifiers.command {
            el = key_icon(el, Icon::Command);
        }
        match shortcut.key.as_str() {
            "enter" | "return" => {
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
                el = key_string(el, &theme, ",");
            }
            "dot" => {
                el = key_string(el, &theme, ".");
            }
            "questionmark" => {
                el = key_string(el, &theme, "?");
            }
            "exclamationmark" => {
                el = key_string(el, &theme, "!");
            }
            "slash" => {
                el = key_string(el, &theme, "/");
            }
            "backslash" => {
                el = key_string(el, &theme, "\\");
            }
            _ => {
                el = key_string(
                    el,
                    &theme,
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
    value: Option<String>,
    items: Vec<(String, String)>,
}

impl Render for Dropdown {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<theme::Theme>();
        let mut el = div().flex().items_center().justify_between();
        if let Some(value) = &self.value {
            el = el.child(div().child(value.clone()).mr_2());
        }
        el = el.child(Img::list_icon(Icon::ChevronDown, None));
        el
    }
}

#[derive(Clone)]
pub struct Actions {
    global: Model<Vec<Action>>,
    local: Model<Vec<Action>>,
    show: bool,
    pub query: Option<TextInput>,
    list: Option<View<List>>,
    update_sender: Sender<bool>,
    pub loading: View<Loading>,
    pub toast: Toast,
    pub dropdown: View<Dropdown>,
}

impl Actions {
    fn new(update_sender: Sender<bool>, cx: &mut WindowContext) -> Self {
        Self {
            global: cx.new_model(|_| Vec::new()),
            local: cx.new_model(|_| Vec::new()),
            show: false,
            query: None,
            list: None,
            loading: Loading::init(cx),
            toast: Toast::init(cx),
            dropdown: cx.new_view(|_| Dropdown {
                value: None,
                items: vec![],
            }),
            update_sender,
        }
    }
    pub fn default(cx: &mut WindowContext) -> Self {
        let (s, r) = channel::<bool>();
        Self::new(s, cx)
    }
    fn combined(&self, cx: &WindowContext) -> Vec<Action> {
        let mut combined = self.local.read(cx).clone();
        combined.append(&mut self.global.read(cx).clone());
        if let Some(action) = combined.get_mut(0) {
            #[cfg(target_os = "macos")]
            let key = "enter";
            #[cfg(not(target_os = "macos"))]
            let key = "return";
            action.shortcut = Some(Shortcut::simple(key));
            combined.push(Action::new(
                Img::list_icon(Icon::BookOpen, None),
                "Actions",
                Some(Shortcut::cmd("k")),
                |this, _| {
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
        let count = list.read(cx).items.len();
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
            .z_index(1000)
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
            if this.items.iter().find(|item| item.0.eq(&value)).is_some() {
                this.value = Some(value);
            }
        });
        self.update()
    }
    pub fn dropdown_cycle(&mut self, cx: &mut WindowContext) {
        self.dropdown.update(cx, |this, cx| {
            if let Some(value) = &this.value {
                let index = this
                    .items
                    .iter()
                    .position(|item| item.0.eq(value))
                    .unwrap_or(0);
                let next = (index + 1) % this.items.len();
                this.value = Some(this.items[next].0.clone());
            }
        });
        self.update()
    }
}

impl Render for Actions {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let combined = self.combined(cx).clone();
        let theme = cx.global::<theme::Theme>();
        if let Some(action) = combined.get(0) {
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
    pub inner: View<Actions>,
}

impl ActionsModel {
    pub fn init(update_sender: Sender<bool>, cx: &mut WindowContext) -> Self {
        let inner = cx.new_view(|cx| Actions::new(update_sender, cx));

        let model = Self {
            inner: inner.clone(),
        };
        inner.update(cx, |this, cx| {
            let query = TextInput::new(cx);
            let actions = this.clone();
            let (_s, r) = channel::<bool>();
            let list = List::new(
                &query,
                &model,
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
                                Some(Item::new(
                                    vec![item.label.clone()],
                                    cx.new_view(|_| {
                                        ListItem::new(
                                            Some(action.image.clone()),
                                            item.label,
                                            None,
                                            accessories,
                                        )
                                    })
                                    .into(),
                                    None,
                                    vec![action],
                                    None,
                                ))
                            })
                            .collect(),
                    ))
                },
                None,
                None,
                r,
                false,
                cx,
            );
            let list_clone = list.clone();
            this.list = Some(list);
            cx.subscribe(&query.view, move |this, _, event, cx| {
                match event {
                    TextEvent::Blur | TextEvent::Back => {
                        this.show = false;
                        cx.notify();
                    }
                    TextEvent::KeyDown(ev) => {
                        #[cfg(target_os = "macos")]
                        let key = "enter";
                        #[cfg(not(target_os = "macos"))]
                        let key = "return";
                        if Shortcut::simple(key).inner.eq(&ev.keystroke) {
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
                        match ev.keystroke.key.as_str() {
                            "escape" => {
                                this.show = false;
                                cx.notify();
                            }
                            _ => {}
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
        model
    }
    pub fn update_global(&self, actions: Vec<Action>, cx: &mut WindowContext) {
        self.inner.update(cx, |model, cx| {
            model.global.update(cx, |this, cx| {
                *this = actions;
                cx.notify();
            });
            model.update_list(cx);
        });
    }
    pub fn update_local(&self, actions: Vec<Action>, cx: &mut WindowContext) {
        self.inner.update(cx, |model, cx| {
            model.local.update(cx, |this, cx| {
                *this = actions;
                cx.notify();
            });
            model.update_list(cx);
        });
    }
}
