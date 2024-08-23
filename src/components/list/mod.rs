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

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    rc::Rc,
    sync::mpsc::channel,
    time::Duration,
};

pub mod nucleo;

use gpui::*;
use log::debug;

use crate::{
    loader::Loader,
    query::{TextEvent, TextInputWeak},
    state::{Action, ActionsModel, Shortcut, StateItem, StateViewContext},
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
                    .font_family(theme.font_mono.clone());
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
    title: SharedString,
    subtitle: Option<SharedString>,
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
            title: title.to_string().into(),
            subtitle: subtitle.map(|s| s.into()),
            img,
            accessories,
        }
    }
}

impl ItemComponent for ListItem {
    fn render(&self, _selected: bool, cx: &WindowContext) -> AnyElement {
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
        .into_any_element()
    }
}

pub trait ItemComponent {
    fn render(&self, selected: bool, cx: &WindowContext) -> AnyElement;
}

pub struct ItemBuilder {
    id: u64,
    preview: Option<(f32, Rc<dyn Preview>)>,
    actions: Vec<Action>,
    weight: Option<u16>,
    keywords: Vec<SharedString>,
    component: Rc<dyn ItemComponent>,
    preset: ItemPreset,
    meta: Option<AnyModel>,
}

impl ItemBuilder {
    pub fn new(id: impl Hash, component: impl ItemComponent + 'static) -> Self {
        let mut s = DefaultHasher::new();
        id.hash(&mut s);
        let id = s.finish();
        Self {
            id,
            preview: None,
            actions: vec![],
            weight: None,
            keywords: vec![],
            meta: None,
            preset: ItemPreset::Default,
            component: Rc::new(component),
        }
    }
    pub fn preview(mut self, width: f32, preview: impl Preview + 'static) -> Self {
        self.preview = Some((width, Rc::new(preview)));
        self
    }
    pub fn meta(mut self, meta: AnyModel) -> Self {
        self.meta = Some(meta);
        self
    }
    pub fn keywords(mut self, keywords: Vec<impl ToString>) -> Self {
        self.keywords = keywords.into_iter().map(|k| k.to_string().into()).collect();
        self
    }
    pub fn actions(mut self, actions: Vec<Action>) -> Self {
        self.actions = actions;
        self
    }
    pub fn weight(mut self, weight: u16) -> Self {
        self.weight = Some(weight);
        self
    }
    pub fn preset(mut self, preset: ItemPreset) -> Self {
        self.preset = preset;
        self
    }
    pub fn build(self) -> Item {
        Item {
            id: self.id,
            preview: self.preview,
            actions: self.actions,
            weight: self.weight,
            keywords: self.keywords,
            selected: false,
            component: self.component,
            meta: self.meta,
            preset: self.preset,
        }
    }
}

#[derive(Clone)]

pub enum ItemPreset {
    Plain,
    Default,
}

#[derive(IntoElement, Clone)]

pub struct Item {
    id: u64,
    preview: Option<(f32, Rc<dyn Preview>)>,
    actions: Vec<Action>,
    weight: Option<u16>,
    keywords: Vec<SharedString>,
    component: Rc<dyn ItemComponent>,
    selected: bool,
    preset: ItemPreset,
    pub meta: Option<AnyModel>,
}

impl Item {
    pub fn get_meta<V: Clone + 'static>(&self, cx: &AppContext) -> Option<V> {
        self.meta
            .clone()
            .and_then(|m| m.downcast::<V>().ok())
            .map(|v| v.read(cx))
            .cloned()
    }
    pub fn get_keywords(&self) -> Vec<SharedString> {
        self.keywords.clone()
    }
}

pub trait Preview: Fn(&mut WindowContext) -> StateItem + 'static {}
impl<F> Preview for F where F: Fn(&mut WindowContext) -> StateItem + 'static {}

impl RenderOnce for Item {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        match self.preset {
            ItemPreset::Plain => self.component.render(self.selected, cx),
            ItemPreset::Default => {
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
                .child(self.component.render(self.selected, cx))
                .into_any_element()
            }
        }
    }
}

type ScrollHandler = Option<Box<dyn FnMut(&ListScrollEvent, &mut WindowContext)>>;

pub struct ListBuilder {
    reverse: bool,
    update_actions: bool,
    interval: Option<Duration>,
    filter: Box<dyn FilterList>,
    scroll_handler: ScrollHandler,
}

impl ListBuilder {
    pub fn new() -> Self {
        Self {
            reverse: false,
            update_actions: true,
            interval: None,
            scroll_handler: None,
            filter: Box::new(|this, cx| {
                let text = this.query.get_text(cx);
                fuzzy_match(&text, this.items_all.clone(), false)
            }),
        }
    }
    pub fn disable_action_updates(mut self) -> Self {
        self.update_actions = false;
        self
    }
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }
    pub fn filter(mut self, filter: impl FilterList + 'static) -> Self {
        self.filter = Box::new(filter);
        self
    }
    pub fn scroll_handler(
        mut self,
        handler: impl FnMut(&ListScrollEvent, &mut WindowContext) + 'static,
    ) -> Self {
        self.scroll_handler = Some(Box::new(handler));
        self
    }
    pub fn build(
        self,
        update: impl UpdateList + 'static,
        context: &mut StateViewContext,
        cx: &mut WindowContext,
    ) -> View<List> {
        List::new(
            Box::new(update),
            self.filter,
            self.interval,
            self.update_actions,
            self.reverse,
            self.scroll_handler,
            context,
            cx,
        )
    }
}

pub trait UpdateList:
    Fn(&mut List, bool, &mut ViewContext<List>) -> anyhow::Result<Option<Vec<Item>>>
{
}
impl<F> UpdateList for F where
    F: Fn(&mut List, bool, &mut ViewContext<List>) -> anyhow::Result<Option<Vec<Item>>>
{
}

pub trait FilterList: Fn(&mut List, &mut ViewContext<List>) -> Vec<Item> {}
impl<F> FilterList for F where F: Fn(&mut List, &mut ViewContext<List>) -> Vec<Item> {}

pub struct List {
    state: ListState,
    selected: Model<u64>,
    pub actions: ActionsModel,
    pub items_all: Vec<Item>,
    pub items: Model<Vec<Item>>,
    pub query: TextInputWeak,
    pub update: Box<dyn UpdateList>,
    pub filter: Box<dyn FilterList>,
    preview: Option<(u64, f32, StateItem)>,
    reverse: bool,
}

impl Render for List {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        //let theme = cx.global::<Theme>();
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

        if self.items.read(cx).is_empty() {
            div()
        } else {
            div()
                .size_full()
                .flex()
                .child(
                    div()
                        .w(width)
                        .h_full()
                        .relative()
                        .child(list(self.state.clone()).size_full().pr_1()),
                )
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
            *this = self
                .items
                .read(cx)
                .get(index)
                .map(|item| item.id)
                .unwrap_or(0);

            cx.notify();
        });
        self.state.scroll_to_reveal_item(index);
    }
    pub fn down(&mut self, cx: &mut ViewContext<Self>) {
        if !self.query.has_focus(cx) {
            return;
        }
        let index = if let Some((index, _)) = self.selected(cx) {
            let i = self.items.read(cx).len() - 1;
            if index < i {
                index + 1
            } else {
                i
            }
        } else {
            0
        };
        self.selected.update(cx, |this, cx| {
            *this = self
                .items
                .read(cx)
                .get(index)
                .map(|item| item.id)
                .unwrap_or(0);
            cx.notify();
        });
        self.state.scroll_to_reveal_item(index);
    }
    pub fn selected(&self, cx: &AppContext) -> Option<(usize, Item)> {
        let id = self.selected.read(cx);

        self.items
            .read(cx)
            .clone()
            .into_iter()
            .enumerate()
            .find(|(_, item)| item.id.eq(id))
    }
    pub fn default_action(&self, cx: &AppContext) -> Option<Action> {
        self.selected(cx)
            .and_then(|(_, item)| item.actions.first().cloned())
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
    pub fn filter(&mut self, _no_scroll: bool, cx: &mut ViewContext<Self>) {
        let filter_fn = std::mem::replace(&mut self.filter, Box::new(|_, _| vec![]));
        let items = filter_fn(self, cx);
        self.filter = filter_fn;

        let mut scroll = self.state.logical_scroll_top();

        self.state.reset(items.len());
        self.items.update(cx, |this, cx| {
            // Determine the ideal scroll position if new elements are added
            if let Some(new_index) = this
                .get(scroll.item_ix)
                .and_then(|scroll_item| items.iter().position(|item| item.id.eq(&scroll_item.id)))
            {
                scroll.item_ix = new_index;
            }

            *this = items;
            cx.notify();
        });

        cx.notify();

        self.state.scroll_to(scroll);

        if self.selected(cx).is_none() {
            self.reset_selection(cx);
        }
    }
    pub fn reset_selection(&mut self, cx: &mut ViewContext<Self>) {
        self.items.update(cx, |items, cx| {
            if items.is_empty() {
                return;
            }
            let s = match self.reverse {
                false => 0,
                true => 1.max(items.len()) - 1,
            };
            self.selected.update(cx, |this, cx| {
                *this = items[s].id;
                cx.notify();
            });
            self.state.scroll_to(ListOffset {
                item_ix: s,
                offset_in_item: Pixels(0.0),
            })
        });
    }
    #[allow(clippy::too_many_arguments)]
    fn new(
        update: Box<dyn UpdateList>,
        filter: Box<dyn FilterList>,
        interval: Option<Duration>,
        update_actions: bool,
        reverse: bool,
        scroll_handler: ScrollHandler,
        context: &mut StateViewContext,
        cx: &mut WindowContext,
    ) -> View<Self> {
        let (selection_sender, r) = channel::<u64>();
        let selected = cx.new_model(|_| 0);
        let items: Model<Vec<Item>> = cx.new_model(|_| vec![]);
        let mut list = Self {
            state: ListState::new(
                0,
                if reverse {
                    ListAlignment::Bottom
                } else {
                    ListAlignment::Top
                },
                Pixels(20.0),
                {
                    let selected = selected.clone();
                    let items = items.clone();
                    let sender = selection_sender.clone();
                    let actions = context.actions.clone();
                    move |i, cx| {
                        let mut item = items.read(cx)[i].clone();
                        let selected = item.id.eq(selected.read(cx));
                        item.selected = selected;
                        let action = item.actions.first().cloned();
                        let actions = actions.inner.upgrade();
                        if actions.is_none() {
                            return div().into_any_element();
                        }
                        let actions = actions.unwrap().read(cx).clone();
                        let sender = sender.clone();
                        let id = item.id;
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
                    }
                },
            ),
            selected,
            items_all: vec![],
            items,
            actions: context.actions.clone(),
            query: context.query.clone(),
            update,
            filter,
            preview: None,
            reverse,
        };
        if let Some(scroll_handler) = scroll_handler {
            list.state.set_scroll_handler(scroll_handler);
        };

        let update_receiver = context.update_receiver.clone();
        let view = cx.new_view(move |cx| {
            cx.observe(&list.selected, move |this: &mut List, _, cx| {
                if let Some((_, selected)) = this.selected(cx) {
                    let preview = if let Some(preview) = selected.preview.as_ref() {
                        if !selected
                            .id
                            .eq(&this.preview.as_ref().map(|p| p.0).unwrap_or_default())
                        {
                            Some((selected.id, preview.0, preview.1(cx)))
                        } else {
                            this.preview.clone()
                        }
                    } else {
                        None
                    };
                    if update_actions {
                        this.actions.update_local(
                            selected.actions.clone(),
                            preview.clone().map(|p| p.2),
                            selected.meta.clone(),
                            cx,
                        );
                    }
                    this.preview = preview;
                } else {
                    if update_actions {
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

                        let update = |this: &mut Self, cx: &mut ViewContext<Self>| {
                            this.update(triggered, cx);
                            last = std::time::Instant::now();
                        };
                        if (poll || triggered) && view.update(&mut cx, update).is_err() {
                            debug!("List view released");
                            break;
                        }
                        cx.background_executor()
                            .timer(Duration::from_millis(50))
                            .await;
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

        if let Some(query) = &context.query.view.upgrade() {
            cx.subscribe(query, move |_subscriber, emitter: &TextEvent, cx| {
                //let clone = clone.clone();
                match emitter {
                    TextEvent::Input { text: _ } => {
                        clone.update(cx, |this, cx| {
                            this.filter(true, cx);
                            this.reset_selection(cx);
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
    pub fn push(&mut self, key: String, item: Item, cx: &mut ViewContext<Self>) {
        let items = self.items.entry(key).or_default();
        // check existing
        if let Some(i) = items.iter().position(|i| i.id.eq(&item.id)) {
            items.remove(i);
        }
        items.push(item);
        if !self.initialized {
            self.initialized = true;
            cx.emit(AsyncListItemsEvent::Initialized);
        };
        cx.emit(AsyncListItemsEvent::Update);
        cx.notify();
    }
    pub fn remove(&mut self, key: String, id: impl Hash, cx: &mut ViewContext<Self>) {
        if let Some(items) = self.items.get_mut(&key) {
            let hash = {
                let mut s = DefaultHasher::new();
                id.hash(&mut s);
                s.finish()
            };
            if let Some(i) = items.iter().position(|i| i.id.eq(&hash)) {
                items.remove(i);
                cx.emit(AsyncListItemsEvent::Update);
                cx.notify();
            }
        }
    }
    pub fn loader(view: &View<Self>, actions: &ActionsModel, cx: &mut WindowContext) {
        if let Some(a) = actions.inner.upgrade() {
            let init = view.read(cx).initialized;
            let a = a.read(cx).clone();
            let mut loader = if !init { Some(Loader::add()) } else { None };
            cx.subscribe(view, move |_, event, _cx| match event {
                AsyncListItemsEvent::Initialized => {
                    if let Some(loader) = loader.as_mut() {
                        loader.remove();
                    }
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
