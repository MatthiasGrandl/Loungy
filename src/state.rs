use gpui::*;

use crate::{
    commands::root::list::RootBuilder,
    icon::Icon,
    list::{Accessory, Img, ImgMask, ImgSize, ImgSource, Item, List, ListItem},
    nucleo::fuzzy_match,
    query::{TextEvent, TextInput},
    theme,
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
        let actions_clone = actions.clone();
        cx.subscribe(&query.view, move |_, event, cx| match event {
            TextEvent::Blur => {
                if !actions_clone.inner.read(cx).show {
                    cx.hide();
                };
            }
            TextEvent::KeyDown(ev) => match ev.keystroke.key.as_str() {
                "escape" => {
                    cx.hide();
                }
                _ => {}
            },
            _ => {}
        })
        .detach();
        let view = view.build(&query, &actions, cx);
        Self {
            query,
            view,
            actions,
        }
    }
}

pub trait StateView {
    fn build(&self, query: &TextInput, actions: &ActionsModel, cx: &mut WindowContext) -> AnyView;
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
        let state = cx.new_model(|_| State { stack: vec![item] });
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

pub trait CloneableFn: Fn(&mut WindowContext) -> () {
    fn clone_box<'a>(&self) -> Box<dyn 'a + CloneableFn>
    where
        Self: 'a;
}

impl<F> CloneableFn for F
where
    F: Fn(&mut WindowContext) -> () + Clone,
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

#[derive(Clone, IntoElement)]
pub struct Action {
    pub label: String,
    pub shortcut: Option<Keystroke>,
    pub image: Img,
    pub action: Box<dyn CloneableFn>,
    pub hide: bool,
}

fn key_icon(el: Div, icon: Icon) -> Div {
    el.child(
        div()
            .child(Img::new(
                ImgSource::Icon(icon),
                ImgMask::Rounded,
                ImgSize::Small,
            ))
            .ml_0p5(),
    )
}

impl RenderOnce for Action {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<theme::Theme>();

        let mut el = div().flex().items_center();
        if let Some(shortcut) = self.shortcut {
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
                _ => {
                    el = el.child(
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
                            .child(shortcut.ime_key.unwrap_or(shortcut.key).to_uppercase())
                            .ml_0p5(),
                    )
                }
            }
        }
        div()
            .ml_auto()
            .child(div().child(self.label).mr_2())
            .flex()
            .items_center()
            .justify_between()
            .child(el)
    }
}

impl Action {
    pub fn new(
        image: Img,
        label: impl ToString,
        shortcut: Option<Keystroke>,
        action: Box<dyn CloneableFn>,
        hide: bool,
    ) -> Self {
        Self {
            label: label.to_string(),
            shortcut,
            action,
            image,
            hide,
        }
    }
}

pub struct Actions {
    global: Vec<Action>,
    local: Vec<Action>,
    combined: Vec<Action>,
    show: bool,
    query: Option<TextInput>,
    list: Option<View<List>>,
}

impl Actions {
    fn compute(&mut self, toggle: Box<dyn CloneableFn>) {
        let mut combined = self.local.clone();
        combined.append(&mut self.global);
        // if there are actions, make the first action the default action
        if let Some(action) = combined.get_mut(0) {
            action.shortcut = Some(Keystroke {
                modifiers: Modifiers::default(),
                key: "enter".to_string(),
                ime_key: None,
            });
            combined.push(Action::new(
                Img::new(
                    ImgSource::Icon(Icon::BookOpen),
                    ImgMask::Rounded,
                    ImgSize::Medium,
                ),
                "Actions",
                Some(Keystroke {
                    modifiers: Modifiers {
                        control: false,
                        alt: false,
                        shift: false,
                        command: true,
                        function: false,
                    },
                    key: "k".to_string(),
                    ime_key: None,
                }),
                toggle,
                true,
            ))
        }
        self.combined = combined;
    }
    fn popup(&mut self, cx: &mut ViewContext<Self>) -> Div {
        if !self.show {
            return div();
        }
        let theme = cx.global::<theme::Theme>();
        let query = self.query.as_ref().unwrap().clone();
        div()
            .absolute()
            .bottom_10()
            .right_0()
            .z_index(1000)
            .w_80()
            .max_h_48()
            .bg(theme.base)
            .rounded_xl()
            .border_1()
            .border_color(theme.crust)
            .shadow_lg()
            .flex()
            .flex_col()
            .child(div().child(self.list.as_ref().unwrap().clone()).p_2())
            .child(
                div()
                    .child(query)
                    .mt_auto()
                    .py_2()
                    .px_4()
                    .border_t_1()
                    .border_color(theme.mantle)
                    .text_base(),
            )
    }
}

impl Render for Actions {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<theme::Theme>();
        if let Some(action) = self.combined.get(0) {
            let open = self.combined.last().unwrap().clone();
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
            div()
        }
    }
}

#[derive(Clone)]
pub struct ActionsModel {
    pub inner: View<Actions>,
    pub toggle: Box<dyn CloneableFn>,
}

impl ActionsModel {
    pub fn init(cx: &mut WindowContext) -> Self {
        let inner = cx.new_view(|_| Actions {
            global: Vec::new(),
            local: Vec::new(),
            combined: Vec::new(),
            show: false,
            query: None,
            list: None,
        });
        let clone = inner.clone();
        let toggle: Box<dyn CloneableFn> = Box::new(move |cx| {
            clone.update(cx, |model, cx| {
                model.show = !model.show;
                cx.notify();
            });
        });
        let model = Self {
            inner: inner.clone(),
            toggle,
        };
        let query = TextInput::new(&model, cx);
        let list = List::new(&query, &model, cx);
        let list_clone = list.clone();
        inner.update(cx, |this, cx| {
            let list = list.clone();
            cx.subscribe(&query.view, move |this, _, event, cx| {
                match event {
                    TextEvent::Blur => {
                        this.show = false;
                        cx.notify();
                    }
                    TextEvent::KeyDown(ev) => match ev.keystroke.key.as_str() {
                        "escape" => {
                            this.show = false;
                            cx.notify();
                        }
                        _ => {}
                    },
                    TextEvent::Input { text } => {
                        let items: Vec<Item> = this
                            .combined
                            .clone()
                            .into_iter()
                            .filter_map(|item| {
                                if item.hide {
                                    return None;
                                }
                                let mut action = item.clone();
                                action.hide = true;
                                Some(Item::new(
                                    vec![item.label.clone()],
                                    cx.new_view(|_| {
                                        ListItem::new(
                                            None,
                                            item.label,
                                            None,
                                            Vec::<Accessory>::new(),
                                        )
                                    })
                                    .into(),
                                    None,
                                    vec![action],
                                    None,
                                ))
                            })
                            .collect();
                        let items = fuzzy_match(text, items, false);
                        list.update(cx, |this, cx| {
                            this.items = items;
                            cx.notify();
                        });
                        cx.notify();
                    }
                    _ => {}
                }
                cx.notify();
            })
            .detach();

            this.query = Some(query);
            this.list = Some(list_clone);
            cx.notify();
        });
        model
    }
    pub fn update_global(&self, actions: Vec<Action>, cx: &mut WindowContext) {
        let toggle = self.toggle.clone();
        self.inner.update(cx, |model, cx| {
            model.global = actions;
            model.compute(toggle);
            cx.notify();
        });
    }
    pub fn update_local(&self, actions: Vec<Action>, cx: &mut WindowContext) {
        let toggle = self.toggle.clone();
        self.inner.update(cx, |model, cx| {
            model.local = actions;
            model.compute(toggle);
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
                    return Some(action.clone());
                }
            }
        }
        None
    }
}
