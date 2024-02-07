use std::{path::PathBuf, sync::Arc};

use gpui::{ImageSource, *};

use crate::{
    icon::Icon,
    query::{TextEvent, TextInput, TextMovement, TextView},
    state::{Action, ActionsModel},
    theme::Theme,
};

#[derive(Clone)]
pub enum ImgMask {
    Circle,
    Rounded,
    None,
}

#[derive(Clone)]
pub enum ImgSource {
    Base(ImageSource),
    Icon(Icon),
}

#[derive(Clone)]
pub enum ImgSize {
    Small,
    Medium,
    Large,
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
    pub fn list_icon(icon: Icon) -> Self {
        Self {
            src: ImgSource::Icon(icon),
            mask: ImgMask::Rounded,
            size: ImgSize::Medium,
        }
    }
    pub fn list_file(src: PathBuf) -> Self {
        Self {
            src: ImgSource::Base(ImageSource::File(Arc::new(src))),
            mask: ImgMask::None,
            size: ImgSize::Medium,
        }
    }
}

impl RenderOnce for Img {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let el = div().overflow_hidden();
        let el = match self.mask {
            ImgMask::Circle => el.rounded_full().bg(theme.surface0),
            ImgMask::Rounded => el.rounded_md().bg(theme.surface0),
            ImgMask::None => el,
        };
        let mut el = match self.size {
            ImgSize::Small => el.size_5(),
            ImgSize::Medium => el.size_6(),
            ImgSize::Large => el.size_8(),
        };
        let img = match self.src {
            ImgSource::Icon(icon) => {
                match self.mask {
                    ImgMask::None => {}
                    _ => {
                        el = el.p_1();
                    }
                }
                let svg = svg().path(icon.path()).text_color(theme.text).size_full();
                svg.into_any_element()
            }
            ImgSource::Base(src) => {
                let img = img(src).size_full();
                img.into_any_element()
            }
        };

        el.child(img)
    }
}

#[derive(Clone, IntoElement)]
pub struct Accessory {
    tag: String,
    img: Option<Img>,
}

impl Accessory {
    pub fn new(tag: impl ToString, img: Option<Img>) -> Self {
        Self {
            tag: tag.to_string(),
            img,
        }
    }
}

impl RenderOnce for Accessory {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let el = div()
            .flex()
            .items_center()
            .text_xs()
            .text_color(theme.subtext0);
        let el = if let Some(img) = self.img {
            el.child(img).mr_1()
        } else {
            el
        };
        el.child(self.tag)
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
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let el = if let Some(img) = &self.img {
            div().child(div().mr_4().child(img.clone()))
        } else {
            div()
        }
        .flex()
        .w_full()
        .items_center()
        .text_sm()
        .child(
            div()
                .child(self.title.clone())
                .font_weight(FontWeight::MEDIUM),
        );
        let el = if let Some(subtitle) = &self.subtitle {
            el.child(subtitle.clone())
        } else {
            el
        };
        el.child(div().ml_auto().children(self.accessories.clone()))
    }
}

#[derive(IntoElement, Clone)]
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
    actions: ActionsModel,
    pub items: Vec<Item>,
    query: TextInput,
}

impl Render for List {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.selection_change(&self.actions, cx);
        let view = cx.view().clone();
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
            .children(
                self.items
                    .clone()
                    .into_iter()
                    .enumerate()
                    .skip(self.skip)
                    .map(|(i, mut item)| {
                        item.selected = i == self.selected;
                        item
                    }),
            )
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
            self.skip = if self.selected > 7 {
                self.selected - 7
            } else {
                0
            };
            cx.notify();
        }
    }
    pub fn new(query: &TextInput, actions: &ActionsModel, cx: &mut WindowContext) -> View<Self> {
        let list = Self {
            selected: 0,
            skip: 0,
            items: vec![],
            actions: actions.clone(),
            query: query.clone(),
        };
        let view = cx.new_view(|_| list);
        let clone = view.clone();

        cx.subscribe(&query.view, move |_subscriber, emitter: &TextEvent, cx| {
            //let clone = clone.clone();
            match emitter {
                TextEvent::Input { text: _ } => {
                    clone.update(cx, |this, cx| {
                        this.selected = 0;
                        this.skip = 0;
                        cx.notify();
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
