use std::{path::PathBuf, sync::Arc};

use gpui::*;

use crate::theme::Theme;

mod icon;

pub use icon::Icon;

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
    pub src: ImgSource,
    pub mask: ImgMask,
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
    pub fn list_url(src: impl ToString) -> Self {
        Self {
            src: ImgSource::Base(ImageSource::Uri(SharedUri::from(src.to_string()))),
            mask: ImgMask::Rounded,
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
                match self.mask {
                    ImgMask::None => {}
                    _ => {
                        el = el.p_0p5();
                    }
                }
                let img = img(src).size_full();
                img.into_any_element()
            }
            ImgSource::Dot(color) => div().rounded_full().bg(color).size_1_2().into_any(),
        };

        el.child(img)
    }
}
