use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

use anyhow::anyhow;
use async_std::{
    channel::{self, Sender},
    fs,
    task::spawn,
};
use gpui::*;
use url::Url;
use website_icon_extract::{ImageLink, ImageType};

use crate::{paths::paths, theme::Theme};

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
    Favicon(View<Favicon>),
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
        if let ImgSource::Favicon(favicon) = &self.src {
            return favicon.clone().into_any_element();
        }
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
                let img = match self.mask {
                    ImgMask::Circle => img.rounded_full().overflow_hidden().bg(theme.surface0),
                    ImgMask::Rounded => img.rounded_md().overflow_hidden().bg(theme.surface0),
                    ImgMask::None => img,
                };
                img.into_any_element()
            }
            ImgSource::Dot(color) => div().rounded_full().bg(color).size_1_2().into_any_element(),
            ImgSource::Favicon(_) => unreachable!(),
        };

        el.child(img).into_any_element()
    }
}

pub struct NoView;
impl Render for NoView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone)]
pub struct Favicon {
    path: PathBuf,
    fallback: Icon,
    valid: bool,
}

impl Favicon {
    async fn fetch_favicon(url: Url, path: PathBuf, sender: Sender<bool>) -> anyhow::Result<()> {
        let list =
            ImageLink::from_website(url, "TEST", 5).map_err(|_| anyhow!("no favicon found"))?;
        let favicon = list
            .into_iter()
            .find(|i| {
                matches!(
                    i.image_type,
                    ImageType::Png | ImageType::Ico | ImageType::Webp
                )
            })
            .ok_or(anyhow!("no favicon found"))?;
        let bytes = reqwest::get(favicon.url).await?.bytes().await?;
        fs::write(path, &bytes).await?;
        sender.send(true).await?;
        Ok(())
    }
    pub fn init(url: impl AsRef<str>, icon: Icon, cx: &mut WindowContext) -> View<Self> {
        cx.new_view(|cx| {
            let mut favicon = Self {
                path: PathBuf::new(),
                fallback: icon,
                valid: false,
            };
            let Ok(url) = Url::parse(url.as_ref()) else {
                return favicon;
            };
            let Some(host) = url.host_str() else {
                return favicon;
            };

            let mut hasher = DefaultHasher::new();
            host.hash(&mut hasher);
            let hash = hasher.finish();
            let cache = paths().cache.join("favicons").join(format!("{}.png", hash));
            favicon.path = cache.clone();

            if cache.exists() {
                favicon.valid = true;
                return favicon;
            };

            let (sender, receiver) = channel::unbounded::<bool>();
            spawn(Self::fetch_favicon(url, cache.clone(), sender));

            cx.spawn(|view, mut cx| async move {
                if let Ok(result) = receiver.recv().await {
                    if result {
                        view.update(&mut cx, |this, cx| {
                            this.valid = true;
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
            favicon
        })
    }
}

impl Render for Favicon {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        if self.valid {
            let mut img = Img::list_file(self.path.clone());
            img.mask = ImgMask::Rounded;
            img
        } else {
            Img::list_icon(self.fallback.clone(), None)
        }
    }
}
