use std::{
    path::PathBuf,
    sync::{mpsc::Receiver, Arc},
    time::Duration,
};

use gpui::{ImageSource, *};

use crate::{
    icon::Icon,
    nucleo::fuzzy_match,
    query::{TextEvent, TextInput},
    state::{Action, ActionsModel, Shortcut},
    theme::Theme,
};

#[derive(Clone)]
#[allow(dead_code)]
pub enum ImgMask {
    Circle,
    Rounded,
    None,
}

#[derive(Clone)]
pub enum ImgSource {
    Base(ImageSource),
    Icon { icon: Icon, color: Option<Hsla> },
    Dot(Hsla),
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum ImgSize {
    XS,
    SM,
    MD,
    LG,
}

#[derive(Clone, IntoElement)]
pub struct Img {
    src: ImgSource,
    mask: ImgMask,
    size: ImgSize,
}

impl Img {
    pub fn new(src: ImgSource, mask: ImgMask, size: ImgSize) -> Self {
        Self { src, mask, size }
    }
    pub fn list_icon(icon: Icon, color: Option<Hsla>) -> Self {
        Self {
            src: ImgSource::Icon { icon, color },
            mask: ImgMask::Rounded,
            size: ImgSize::MD,
        }
    }
    pub fn accessory_icon(icon: Icon, color: Option<Hsla>) -> Self {
        Self {
            src: ImgSource::Icon { icon, color },
            mask: ImgMask::None,
            size: ImgSize::SM,
        }
    }
    pub fn list_file(src: PathBuf) -> Self {
        Self {
            src: ImgSource::Base(ImageSource::File(Arc::new(src))),
            mask: ImgMask::None,
            size: ImgSize::MD,
        }
    }
    pub fn list_dot(color: Hsla) -> Self {
        Self {
            src: ImgSource::Dot(color),
            mask: ImgMask::None,
            size: ImgSize::MD,
        }
    }
}

impl RenderOnce for Img {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let el = div()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center();
        let el = match self.mask {
            ImgMask::Circle => el.rounded_full().bg(theme.surface0),
            ImgMask::Rounded => el.rounded_md().bg(theme.surface0),
            ImgMask::None => el,
        };
        let mut el = match self.size {
            ImgSize::XS => el.size_4(),
            ImgSize::SM => el.size_5(),
            ImgSize::MD => el.size_6(),
            ImgSize::LG => el.size_8(),
        };
        let img = match self.src {
            ImgSource::Icon { icon, color } => {
                match self.mask {
                    ImgMask::None => {}
                    _ => {
                        el = el.p_1();
                    }
                }
                let svg = svg()
                    .path(icon.path())
                    .text_color(color.unwrap_or(theme.text))
                    .size_full();
                svg.into_any_element()
            }
            ImgSource::Base(src) => {
                let img = img(src).size_full();
                img.into_any_element()
            }
            ImgSource::Dot(color) => div().rounded_full().bg(color).size_2_3().into_any(),
        };

        el.child(img)
    }
}

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
    preview: Option<AnyView>,
    actions: Vec<Action>,
    pub weight: Option<u16>,
    selected: bool,
}

impl Item {
    pub fn new(
        keywords: Vec<impl ToString>,
        component: AnyView,
        preview: Option<AnyView>,
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

pub struct List {
    selected: usize,
    skip: usize,
    update_actions: bool,
    pub actions: ActionsModel,
    pub items_all: Vec<Item>,
    pub items: Vec<Item>,
    pub query: TextInput,
    pub update: Box<dyn Fn(&mut Self, &mut ViewContext<Self>) -> Vec<Item>>,
    pub filter: Box<dyn Fn(&mut Self, &mut ViewContext<Self>) -> Vec<Item>>,
}

impl Render for List {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        if self.update_actions {
            self.selection_change(&self.actions, cx);
        }
        let view = cx.view().clone();
        let mut items: Vec<(usize, Item)> = self
            .items
            .clone()
            .into_iter()
            .enumerate()
            .skip(self.skip)
            .collect();
        items.truncate(8);
        div()
            .w_full()
            .on_scroll_wheel(move |ev, cx| {
                view.update(cx, |this, cx| {
                    let y = ev.delta.pixel_delta(Pixels(1.0)).y.0;
                    if y > 10.0 {
                        this.up(cx);
                    } else if y < -10.0 {
                        this.down(cx);
                    }
                });
            })
            .children(items.into_iter().map(|(i, mut item)| {
                item.selected = i == self.selected;
                div().child(item.clone()).on_mouse_down(MouseButton::Left, {
                    //
                    let actions = self.actions.inner.read(cx).clone();
                    let action = item.actions.first().cloned();
                    move |_, cx| {
                        let mut actions = actions.clone();
                        if let Some(action) = &action {
                            (action.action)(&mut actions, cx);
                        }
                    }
                })
            }))
    }
}

impl List {
    pub fn up(&mut self, cx: &mut ViewContext<Self>) {
        if !self.query.has_focus(cx) {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
            self.skip = if self.skip > self.selected {
                self.selected
            } else {
                self.skip
            };
            cx.notify();
        }
    }
    pub fn down(&mut self, cx: &mut ViewContext<Self>) {
        if !self.query.has_focus(cx) {
            return;
        }
        if self.selected < self.items.len() - 1 {
            self.selected += 1;
            self.skip = if self.selected > self.skip + 7 {
                self.selected - 7
            } else {
                self.skip
            };
            cx.notify();
        }
    }
    pub fn selected(&self) -> Option<&Item> {
        self.items.get(self.selected)
    }
    pub fn default_action(&self) -> Option<&Action> {
        self.selected().and_then(|item| item.actions.first())
    }
    pub fn update(&mut self, cx: &mut ViewContext<Self>) {
        let update_fn = std::mem::replace(&mut self.update, Box::new(|_, _| vec![]));
        self.items_all = update_fn(self, cx);
        self.update = update_fn;
        self.filter(cx);
    }
    pub fn filter(&mut self, cx: &mut ViewContext<Self>) {
        let filter_fn = std::mem::replace(&mut self.filter, Box::new(|_, _| vec![]));
        self.items = filter_fn(self, cx);
        self.filter = filter_fn;
        cx.notify();
    }
    pub fn new(
        query: &TextInput,
        actions: &ActionsModel,
        update: impl Fn(&mut Self, &mut ViewContext<Self>) -> Vec<Item> + 'static,
        filter: Option<Box<dyn Fn(&mut Self, &mut ViewContext<Self>) -> Vec<Item>>>,
        interval: Option<Duration>,
        update_receiver: Receiver<bool>,
        update_actions: bool,
        cx: &mut WindowContext,
    ) -> View<Self> {
        let mut list = Self {
            selected: 0,
            skip: 0,
            items_all: vec![],
            items: vec![],
            actions: actions.clone(),
            query: query.clone(),
            update: Box::new(update),
            update_actions,
            filter: filter.unwrap_or(Box::new(|this, cx| {
                let text = this.query.clone().view.read(cx).text.clone();
                fuzzy_match(&text, this.items_all.clone(), false)
            })),
        };
        let view = cx.new_view(|cx| {
            cx.spawn(|view, mut cx| async move {
                let mut last = std::time::Instant::now();
                loop {
                    if let Some(view) = view.upgrade() {
                        let poll = interval.map(|i| last.elapsed() > i).unwrap_or(false);
                        if poll || update_receiver.try_recv().is_ok() {
                            let _ = view.update(&mut cx, |this: &mut Self, cx| {
                                if this.query.has_focus(cx) {
                                    this.update(cx);
                                    last = std::time::Instant::now();
                                }
                            });
                        }
                        cx.background_executor()
                            .timer(Duration::from_millis(50))
                            .await;
                    } else {
                        break;
                    }
                }
            })
            .detach();
            list.update(cx);
            list
        });
        let clone = view.clone();

        cx.subscribe(&query.view, move |_subscriber, emitter: &TextEvent, cx| {
            //let clone = clone.clone();
            match emitter {
                TextEvent::Input { text: _ } => {
                    clone.update(cx, |this, cx| {
                        this.selected = 0;
                        this.skip = 0;
                        this.filter(cx);
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
        if let Some(item) = self.items.get(self.selected) {
            actions.update_local(item.actions.clone(), cx)
        }
    }
}
