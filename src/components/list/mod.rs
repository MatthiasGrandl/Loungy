use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};

pub mod nucleo;

use async_std::task::sleep;

use gpui::*;
use log::debug;

use crate::{
    query::{TextEvent, TextInput, TextInputWeak},
    state::{Action, ActionsModel, Loading, Shortcut, StateItem},
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
    id: u64,
    pub keywords: Vec<String>,
    component: AnyView,
    pub preview: Option<(f32, Box<dyn Preview>)>,
    actions: Vec<Action>,
    pub weight: Option<u16>,
    selected: bool,
    pub meta: Box<dyn Meta>,
    render: Option<fn(Self, bool, &WindowContext) -> Div>,
}

pub trait Meta: std::any::Any {
    fn clone_box(&self) -> Box<dyn Meta>;
    fn value(&self) -> &dyn std::any::Any;
}

impl<F> Meta for F
where
    F: Clone + std::any::Any,
{
    fn clone_box(&self) -> Box<dyn Meta> {
        Box::new(self.clone())
    }
    fn value(&self) -> &dyn std::any::Any {
        self
    }
}

impl<'a> Clone for Box<dyn 'a + Meta> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

pub trait Preview: Fn(&mut WindowContext) -> StateItem {
    fn clone_box<'a>(&self) -> Box<dyn 'a + Preview>
    where
        Self: 'a;
}

impl<F> Preview for F
where
    F: Fn(&mut WindowContext) -> StateItem + Clone,
{
    fn clone_box<'a>(&self) -> Box<dyn 'a + Preview>
    where
        Self: 'a,
    {
        Box::new(self.clone())
    }
}

impl<'a> Clone for Box<dyn 'a + Preview> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

impl Item {
    pub fn new<T: Hash>(
        t: T,
        keywords: Vec<impl ToString>,
        component: AnyView,
        preview: Option<(f32, Box<dyn Preview>)>,
        actions: Vec<Action>,
        weight: Option<u16>,
        meta: Option<Box<dyn Meta>>,
        render: Option<fn(Self, bool, &WindowContext) -> Div>,
    ) -> Self {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        let id = s.finish();
        Self {
            id,
            keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
            component,
            preview,
            actions,
            weight,
            selected: false,
            meta: meta.unwrap_or_else(|| Box::new(())),
            render,
        }
    }
}

impl RenderOnce for Item {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        if let Some(render) = &self.render {
            render(self.clone(), self.selected, cx)
        } else {
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
}

pub struct List {
    state: ListState,
    selected: Model<u64>,
    update_actions: bool,
    selection_sender: Sender<u64>,
    pub actions: ActionsModel,
    pub items_all: Vec<Item>,
    pub items: Vec<Item>,
    pub query: TextInputWeak,
    pub update:
        Box<dyn Fn(&mut Self, bool, &mut ViewContext<Self>) -> anyhow::Result<Option<Vec<Item>>>>,
    pub filter: Box<dyn Fn(&mut Self, &mut ViewContext<Self>) -> Vec<Item>>,
    preview: Option<(u64, f32, StateItem)>,
    alignment: ListAlignment,
}

impl Render for List {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let (width, preview) = self
            .preview
            .as_ref()
            .map(|p| {
                (
                    relative(1.0 - p.1),
                    div().child(p.2.view.clone()).w(relative(p.1)).pl_1(),
                )
            })
            .unwrap_or((relative(1.0), div()));

        if self.items.len() == 0 {
            div()
        } else {
            div()
                .size_full()
                .flex()
                .child(list(self.state.clone()).w(width).pr_1().h_full())
                .child(preview)
        }
    }
}

impl List {
    pub fn up(&mut self, cx: &mut ViewContext<Self>) {
        if !self.query.has_focus(cx) {
            return;
        }
        let index = if let Some((index, _)) = self.selected(cx) {
            if index > 0 {
                index - 1
            } else {
                0
            }
        } else {
            0
        };

        self.selected.update(cx, |this, cx| {
            *this = self.items[index].id;

            cx.notify();
        });
        self.state.scroll_to_reveal_item(index);
    }
    pub fn down(&mut self, cx: &mut ViewContext<Self>) {
        if !self.query.has_focus(cx) {
            return;
        }
        let index = if let Some((index, _)) = self.selected(cx) {
            if index < self.items.len() - 1 {
                index + 1
            } else {
                self.items.len() - 1
            }
        } else {
            0
        };
        self.selected.update(cx, |this, cx| {
            *this = self.items[index].id;
            cx.notify();
        });
        self.state.scroll_to_reveal_item(index);
    }
    pub fn selected(&self, cx: &AppContext) -> Option<(usize, &Item)> {
        let id = self.selected.read(cx);
        self.items
            .iter()
            .enumerate()
            .find(|(_, item)| item.id.eq(id))
    }
    pub fn default_action(&self, cx: &AppContext) -> Option<&Action> {
        self.selected(cx).and_then(|(_, item)| item.actions.first())
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
                let _ = self.actions.inner.update(cx, |this, cx| {
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
            self.alignment.clone(),
            Pixels(20.0),
            move |i, cx| {
                let mut item = items[i].clone();
                let selected = item.id.eq(s.read(cx));
                item.selected = selected;
                let action = item.actions.first().cloned();
                let actions = actions.inner.upgrade();
                if actions.is_none() {
                    return div().into_any_element();
                }
                let actions = actions.unwrap().read(cx).clone();
                let sender = sender.clone();
                let id = item.id.clone();
                div()
                    .child(item)
                    .on_mouse_down(MouseButton::Left, {
                        move |ev, cx| match ev.click_count {
                            1 => {
                                let _ = sender.send(id);
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
        if self.selected(cx).is_none() {
            self.change_alignment(self.alignment, cx);
        }
    }
    pub fn change_alignment(&mut self, alignment: ListAlignment, cx: &mut ViewContext<Self>) {
        self.alignment = alignment;
        if self.items.is_empty() {
            return;
        }
        let s = match self.alignment {
            ListAlignment::Top => 0,
            ListAlignment::Bottom => 1.max(self.items.len()) - 1,
        };
        self.selected.update(cx, |this, cx| {
            *this = self.items[s].id;
            cx.notify();
        });
    }
    pub fn new(
        query: &TextInputWeak,
        actions: &ActionsModel,
        update: impl Fn(&mut Self, bool, &mut ViewContext<Self>) -> anyhow::Result<Option<Vec<Item>>>
            + 'static,
        filter: Option<Box<dyn Fn(&mut Self, &mut ViewContext<Self>) -> Vec<Item>>>,
        interval: Option<Duration>,
        update_receiver: Receiver<bool>,
        update_actions: bool,
        cx: &mut WindowContext,
    ) -> View<Self> {
        let (selection_sender, r) = channel::<u64>();
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
                let text = this.query.get_text(cx);
                fuzzy_match(&text, this.items_all.clone(), false)
            })),
            update_actions,
            selection_sender,
            preview: None,
            alignment: ListAlignment::Top,
        };

        let view = cx.new_view(|cx| {
            cx.observe(&list.selected, |this: &mut List, _, cx| {
                if let Some((_, selected)) = this.selected(cx) {
                    let preview = if let Some(preview) = selected.preview.as_ref() {
                        if !selected.id.eq(&this
                            .preview
                            .as_ref()
                            .map(|p| p.0.clone())
                            .unwrap_or_default())
                        {
                            Some((selected.id, preview.0.clone(), preview.1(cx)))
                        } else {
                            this.preview.clone()
                        }
                    } else {
                        None
                    };
                    if this.update_actions {
                        this.actions.update_local(
                            selected.actions.clone(),
                            preview.clone().map(|p| p.2),
                            cx,
                        );
                    }
                    this.preview = preview;
                } else {
                    if this.update_actions {
                        this.actions.clear_local(cx);
                    }
                    this.preview = None;
                }
                cx.notify();
            })
            .detach();
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
                            if view
                                .update(&mut cx, |this: &mut Self, cx| {
                                    this.update(triggered, cx);
                                    last = std::time::Instant::now();
                                })
                                .is_err()
                            {
                                debug!("List view released");
                                break;
                            }
                        }
                        sleep(Duration::from_millis(50)).await;
                        // cx.background_executor()
                        //     .timer(Duration::from_millis(50))
                        //     .await;
                    } else {
                        debug!("List view released");
                        break;
                    }
                }
            })
            .detach();
            list.update(true, cx);
            list
        });
        let clone = view.clone();

        if let Some(query) = &query.view.upgrade() {
            cx.subscribe(query, move |_subscriber, emitter: &TextEvent, cx| {
                //let clone = clone.clone();
                match emitter {
                    TextEvent::Input { text: _ } => {
                        clone.update(cx, |this, cx| {
                            this.filter(true, cx);
                            this.change_alignment(this.alignment, cx);
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
        }
        view
    }
}

pub struct AsyncListItems {
    pub items: HashMap<String, Vec<Item>>,
    pub initialized: bool,
}

impl AsyncListItems {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            initialized: false,
        }
    }
    pub fn update(&mut self, key: String, items: Vec<Item>, cx: &mut ViewContext<Self>) {
        self.items.insert(key, items);
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
        if let Some(a) = actions.inner.upgrade() {
            let mut a = a.read(cx).clone();
            Loading::update(&mut a.loading, true, cx);
            cx.subscribe(view, move |_, event, cx| match event {
                AsyncListItemsEvent::Initialized => {
                    Loading::update(&mut a.loading, false, cx);
                }
                AsyncListItemsEvent::Update => {
                    a.update();
                }
            })
            .detach();
        }
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
