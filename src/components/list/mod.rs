use std::{
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};

pub mod nucleo;

use async_std::task::sleep;
use gpui::*;

use crate::{
    query::{TextEvent, TextInput},
    state::{Action, ActionsModel, Shortcut, StateItem},
    theme::Theme,
};

use nucleo::fuzzy_match;

use super::shared::{Img, ImgSource};

#[derive(Clone, IntoElement)]
pub enum Accessory {
    Tag { tag: String, img: Option<Img> },
    Shortcut(Shortcut),
}

impl Accessory {
    pub fn new(tag: impl ToString, img: Option<Img>) -> Self {
        Self::Tag {
            tag: tag.to_string(),
            img,
        }
    }
    pub fn shortcut(shortcut: Shortcut) -> Self {
        Self::Shortcut(shortcut)
    }
}

impl RenderOnce for Accessory {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        match self {
            Accessory::Tag { tag, img } => {
                let el = div()
                    .flex()
                    .items_center()
                    .text_color(theme.subtext0)
                    .font(theme.font_mono.clone());
                let el = if let Some(mut img) = img {
                    img.src = match img.src {
                        ImgSource::Icon { icon, color } => ImgSource::Icon {
                            icon,
                            color: color.or(Some(theme.subtext0)),
                        },
                        src => src,
                    };
                    el.child(div().mr_3().child(img))
                } else {
                    el
                };
                el.child(tag).ml_6()
            }
            Accessory::Shortcut(shortcut) => div().child(shortcut),
        }
    }
}

#[derive(Clone)]
pub struct ListItem {
    title: String,
    subtitle: Option<String>,
    img: Option<Img>,
    accessories: Vec<Accessory>,
}

impl ListItem {
    pub fn new(
        img: Option<Img>,
        title: impl ToString,
        subtitle: Option<String>,
        accessories: Vec<Accessory>,
    ) -> Self {
        Self {
            title: title.to_string(),
            subtitle,
            img,
            accessories,
        }
    }
}

impl Render for ListItem {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let el = if let Some(img) = &self.img {
            div().child(div().mr_4().child(img.clone()))
        } else {
            div()
        }
        .flex()
        .w_full()
        .items_center()
        .text_xs()
        .child(
            div()
                .text_sm()
                .child(self.title.clone())
                .font_weight(FontWeight::MEDIUM),
        );
        let el = if let Some(subtitle) = &self.subtitle {
            el.child(
                div()
                    .ml_2()
                    .text_color(theme.subtext0)
                    .child(subtitle.clone()),
            )
        } else {
            el
        };
        el.child(
            div()
                .flex()
                .items_center()
                .ml_auto()
                .children(self.accessories.clone()),
        )
    }
}

#[derive(IntoElement, Clone)]
#[allow(dead_code)]
pub struct Item {
    pub keywords: Vec<String>,
    component: AnyView,
    preview: Option<StateItem>,
    actions: Vec<Action>,
    pub weight: Option<u16>,
    selected: bool,
}

impl Item {
    pub fn new(
        keywords: Vec<impl ToString>,
        component: AnyView,
        preview: Option<StateItem>,
        actions: Vec<Action>,
        weight: Option<u16>,
    ) -> Self {
        Self {
            keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
            component,
            preview,
            actions,
            weight,
            selected: false,
        }
    }
}

impl RenderOnce for Item {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let mut bg_hover = theme.mantle;
        bg_hover.fade_out(0.5);
        if self.selected {
            div().border_color(theme.crust).bg(theme.mantle)
        } else {
            div().hover(|s| s.bg(bg_hover))
        }
        .p_2()
        .border_1()
        .rounded_xl()
        .child(self.component)
    }
}

// pub struct ListStateInner {
//     pub items: Vec<Item>,
//     pub selected: usize,
// }

pub struct List {
    state: ListState,
    selected: Model<usize>,
    update_actions: bool,
    selection_sender: Sender<usize>,
    pub actions: ActionsModel,
    pub items_all: Vec<Item>,
    pub items: Vec<Item>,
    pub query: TextInput,
    pub update:
        Box<dyn Fn(&mut Self, bool, &mut ViewContext<Self>) -> anyhow::Result<Option<Vec<Item>>>>,
    pub filter: Box<dyn Fn(&mut Self, &mut ViewContext<Self>) -> Vec<Item>>,
}

impl Render for List {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        if self.update_actions {
            self.selection_change(&self.actions, cx);
        }

        let selected = self.selected(cx);
        let preview = selected
            .map(|s| {
                s.preview
                    .clone()
                    .map(|p| div().child(p.view.clone()).w_1_2().pl_1())
            })
            .flatten();

        if self.items.len() == 0 {
            div()
        } else {
            div()
                .size_full()
                .flex()
                .child(if preview.is_some() {
                    list(self.state.clone()).w_1_2().pr_1().h_full()
                } else {
                    list(self.state.clone()).size_full()
                })
                .child(preview.unwrap_or(div()))
        }
    }
}

impl List {
    pub fn up(&mut self, cx: &mut ViewContext<Self>) {
        if !self.query.has_focus(cx) {
            return;
        }
        self.selected.update(cx, |this, cx| {
            if *this > 0 {
                *this -= 1;
                self.state.scroll_to_reveal_item(*this);
                cx.notify();
            }
        });
    }
    pub fn down(&mut self, cx: &mut ViewContext<Self>) {
        if !self.query.has_focus(cx) {
            return;
        }
        self.selected.update(cx, |this, cx| {
            if *this < self.items.len() - 1 {
                *this += 1;
                self.state.scroll_to_reveal_item(*this);
                cx.notify();
            }
        });
    }
    pub fn selected(&self, cx: &AppContext) -> Option<&Item> {
        self.items.get(*self.selected.read(cx))
    }
    pub fn default_action(&self, cx: &AppContext) -> Option<&Action> {
        self.selected(cx).and_then(|item| item.actions.first())
    }
    pub fn update(&mut self, no_scroll: bool, cx: &mut ViewContext<Self>) {
        let update_fn = std::mem::replace(&mut self.update, Box::new(|_, _, _| Ok(None)));
        let result = update_fn(self, no_scroll, cx);
        self.update = update_fn;
        match result {
            Ok(Some(items)) => {
                self.items_all = items;
                self.filter(no_scroll, cx);
            }
            Ok(None) => {}
            Err(_err) => {
                self.actions.inner.update(cx, |this, cx| {
                    this.toast.error("Failed to refresh list", cx);
                });
            }
        }
    }
    pub fn filter(&mut self, no_scroll: bool, cx: &mut ViewContext<Self>) {
        let filter_fn = std::mem::replace(&mut self.filter, Box::new(|_, _| vec![]));
        self.items = filter_fn(self, cx);
        self.filter = filter_fn;

        let items = self.items.clone();
        let s = self.selected.clone();
        let actions = self.actions.clone();
        let sender = self.selection_sender.clone();

        let scroll = self.state.logical_scroll_top().clone();
        self.state = ListState::new(
            self.items.len(),
            ListAlignment::Top,
            Pixels(20.0),
            move |i, cx| {
                let selected = i.eq(s.read(cx));
                let mut item = items[i].clone();
                item.selected = selected;
                let action = item.actions.first().cloned();
                let actions = actions.inner.read(cx).clone();
                let sender = sender.clone();
                div()
                    .child(item)
                    .on_mouse_down(MouseButton::Left, {
                        move |ev, cx| match ev.click_count {
                            1 => {
                                let _ = sender.send(i);
                            }
                            2 => {
                                let mut actions = actions.clone();
                                if let Some(action) = &action {
                                    (action.action)(&mut actions, cx);
                                }
                            }
                            _ => {}
                        }
                    })
                    .into_any_element()
            },
        );
        if !no_scroll {
            self.state.scroll_to(scroll);
        }
        cx.notify();
    }
    pub fn new(
        query: &TextInput,
        actions: &ActionsModel,
        update: impl Fn(&mut Self, bool, &mut ViewContext<Self>) -> anyhow::Result<Option<Vec<Item>>>
            + 'static,
        filter: Option<Box<dyn Fn(&mut Self, &mut ViewContext<Self>) -> Vec<Item>>>,
        interval: Option<Duration>,
        update_receiver: Receiver<bool>,
        update_actions: bool,
        cx: &mut WindowContext,
    ) -> View<Self> {
        let (selection_sender, r) = channel::<usize>();
        let mut list = Self {
            state: ListState::new(0, ListAlignment::Top, Pixels(20.0), move |_, _| {
                div().into_any_element()
            }),
            selected: cx.new_model(|_| 0),
            items_all: vec![],
            items: vec![],
            actions: actions.clone(),
            query: query.clone(),
            update: Box::new(update),
            filter: filter.unwrap_or(Box::new(|this, cx| {
                let text = this.query.clone().view.read(cx).text.clone();
                fuzzy_match(&text, this.items_all.clone(), false)
            })),
            update_actions,
            selection_sender,
        };
        let view = cx.new_view(|cx| {
            cx.spawn(|view, mut cx| async move {
                let mut last = std::time::Instant::now();
                loop {
                    if let Some(view) = view.upgrade() {
                        let poll = interval.map(|i| last.elapsed() > i).unwrap_or(false);
                        if let Ok(selected) = r.try_recv() {
                            let _ = view.update(&mut cx, |this: &mut Self, cx| {
                                this.selected.update(cx, |this, cx| {
                                    *this = selected;
                                    cx.notify();
                                });
                                cx.notify();
                            });
                        }
                        let triggered = update_receiver.try_recv().is_ok();

                        if poll || triggered {
                            let _ = view.update(&mut cx, |this: &mut Self, cx| {
                                if this.query.has_focus(cx)
                                    || this
                                        .actions
                                        .inner
                                        .read(cx)
                                        .query
                                        .as_ref()
                                        .unwrap()
                                        .has_focus(cx)
                                {
                                    this.update(triggered, cx);
                                    last = std::time::Instant::now();
                                }
                            });
                        }
                        sleep(Duration::from_millis(50)).await;
                        // cx.background_executor()
                        //     .timer(Duration::from_millis(50))
                        //     .await;
                    } else {
                        eprintln!("update list view dropped");
                        break;
                    }
                }
            })
            .detach();
            list.update(true, cx);
            list
        });
        let clone = view.clone();

        cx.subscribe(&query.view, move |_subscriber, emitter: &TextEvent, cx| {
            //let clone = clone.clone();
            match emitter {
                TextEvent::Input { text: _ } => {
                    clone.update(cx, |this, cx| {
                        this.selected.update(cx, |this, cx| {
                            *this = 0;
                            cx.notify();
                        });
                        this.filter(true, cx);
                    });
                }
                TextEvent::KeyDown(ev) => match ev.keystroke.key.as_str() {
                    "up" => {
                        clone.update(cx, |this, cx| {
                            this.up(cx);
                        });
                    }
                    "down" => {
                        clone.update(cx, |this, cx| {
                            this.down(cx);
                        });
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .detach();
        view
    }
    pub fn selection_change(&self, actions: &ActionsModel, cx: &mut WindowContext) {
        if let Some(item) = self.selected(cx) {
            actions.update_local(item.actions.clone(), cx)
        } else {
            actions.update_local(vec![], cx)
        }
    }
}

pub struct AsyncListItems {
    pub items: Vec<Item>,
    pub initialized: bool,
}

impl AsyncListItems {
    pub fn update(&mut self, items: Vec<Item>, cx: &mut ViewContext<Self>) {
        self.items = items;
        if !self.initialized {
            self.initialized = true;
            cx.emit(AsyncListItemsEvent::Initialized);
        };
        cx.emit(AsyncListItemsEvent::Update);
        cx.notify();
    }
    pub fn loader(view: &View<Self>, actions: &ActionsModel, cx: &mut WindowContext) {
        if view.read(cx).initialized {
            return;
        }
        let loading = actions.inner.read(cx).loading.clone();
        loading.update(cx, |this, _| {
            this.inner = true;
        });
        cx.subscribe(view, move |_, event, cx| match event {
            AsyncListItemsEvent::Initialized => {
                loading.update(cx, |this, _| {
                    this.inner = false;
                });
            }
            _ => {}
        })
        .detach();
    }
}

impl Render for AsyncListItems {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        div()
    }
}

pub enum AsyncListItemsEvent {
    Initialized,
    Update,
}

impl EventEmitter<AsyncListItemsEvent> for AsyncListItems {}
