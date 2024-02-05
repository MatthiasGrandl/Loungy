use std::path::PathBuf;

use gpui::{ImageSource, *};

use crate::{
    icon::Icon,
    query::{self, TextEvent, TextMovement},
    theme::Theme,
    workspace::Query,
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
}

impl RenderOnce for Img {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let el = div();

        let el = match self.mask {
            ImgMask::Circle => el.rounded_full().bg(theme.mantle),
            ImgMask::Rounded => el.rounded_lg().bg(theme.mantle),
            ImgMask::None => el,
        };
        let img = match self.src {
            ImgSource::Icon(icon) => img(""),
            ImgSource::Base(src) => img(src).w_8().h_8(),
        };
        let img = match self.size {
            ImgSize::Small => img.w_4().h_4(),
            ImgSize::Medium => img.w_6().h_6(),
            ImgSize::Large => img.w_8().w_8(),
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
        .text_sm();
        let el = if let Some(subtitle) = &self.subtitle {
            el.child(subtitle.clone())
        } else {
            el
        };
        el.child(self.title.clone())
            .child(div().ml_auto().children(self.accessories.clone()))
    }
}

#[derive(Clone)]
pub struct Action {
    label: String,
    shortcut: Option<Keystroke>,
    action: fn(&WindowContext),
    image: Img,
}

impl Action {
    pub fn new(
        image: Img,
        label: impl ToString,
        shortcut: Option<Keystroke>,
        action: fn(&WindowContext),
    ) -> Self {
        Self {
            label: label.to_string(),
            shortcut,
            action,
            image,
        }
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
    pub items: Vec<Item>,
}

impl Render for List {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .w_full()
            .on_scroll_wheel(|ev, cx| {
                cx.update_global::<Query, _>(|query, cx| {
                    query.inner.update(cx, |_model, cx| {
                        let y = ev.delta.pixel_delta(Pixels(1.0)).y.0;
                        if y > 10.0 {
                            cx.emit(TextEvent::Movement(TextMovement::Up));
                        } else if y < -10.0 {
                            cx.emit(TextEvent::Movement(TextMovement::Down));
                        }
                    });
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
    pub fn new(cx: &mut WindowContext) -> View<Self> {
        let view = cx.new_view(|_cx| Self {
            selected: 0,
            skip: 0,
            items: vec![],
        });
        let clone = view.clone();
        cx.update_global::<Query, _>(|query, cx| {
            cx.subscribe(&query.inner, move |_subscriber, emitter: &TextEvent, cx| {
                let clone = clone.clone();
                match emitter {
                    TextEvent::Input { text: _ } => {
                        clone.update(cx, |this, cx| {
                            this.selected = 0;
                            this.skip = 0;
                            cx.notify();
                        });
                    }
                    TextEvent::Movement(TextMovement::Up) => {
                        clone.update(cx, |this, cx| {
                            if this.selected > 0 {
                                this.selected -= 1;
                                this.skip = if this.skip > this.selected {
                                    this.selected
                                } else {
                                    this.skip
                                };
                                cx.notify();
                            }
                        });
                    }
                    TextEvent::Movement(TextMovement::Down) => {
                        clone.update(cx, |this, cx| {
                            if this.selected < this.items.len() - 1 {
                                this.selected += 1;
                                this.skip = if this.selected > 7 {
                                    this.selected - 7
                                } else {
                                    0
                                };
                                cx.notify();
                            }
                        });
                    }
                    TextEvent::Submit {} => {
                        clone.update(cx, |this, cx| {
                            (this.items[this.selected].actions[0].action)(cx);
                        });
                    }
                    _ => {}
                }
            })
            .detach();
        });
        view
    }
}
