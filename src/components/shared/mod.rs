use std::{cell::OnceCell, path::PathBuf, sync::Arc};

use anyhow::anyhow;
use async_std::{
    fs,
    task::{spawn, spawn_blocking},
};
use gpui::*;
use image::ImageFormat;
use log::debug;
use url::Url;

use crate::{paths::paths, theme::Theme};
use scraper::{Html, Selector};

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
    url: String,
    fallback: Icon,
    path: Option<PathBuf>,
    init: OnceCell<bool>,
}

impl Favicon {
    async fn fetch_favicon(url: Url, cache: PathBuf) -> anyhow::Result<PathBuf> {
        let base_url = Url::parse(&format!("{}://{}", url.scheme(), url.host_str().unwrap()))?;
        let mut targets = vec![url.clone(), base_url.clone()];
        // if subdomain
        if let Some(domain) = base_url.domain() {
            let split: Vec<&str> = domain.split('.').collect();
            if split.len() > 2 {
                targets.push(Url::parse(&format!(
                    "{}://{}",
                    url.scheme(),
                    split[split.len() - 2..split.len()].join(".")
                ))?);
            }
        };
        let client = reqwest::ClientBuilder::new()
            .user_agent("favicon_crawler (loungy.app)")
            .build()?;
        for target in targets {
            let response = client.get(target.clone()).send().await?;
            let url = response.url().clone();
            let html = response.text().await?;

            let mut hrefs: Vec<String> = spawn_blocking(move || {
                let document = Html::parse_document(&html);
                let selector = Selector::parse("link[rel~='icon'], link[rel~='shortcut icon'], link[rel~='alternate icon'], link[rel~='apple-touch-icon'], link[rel~='apple-touch-icon-precomposed']").unwrap();

                document
                    .select(&selector)
                    .filter_map(|element| element.value().attr("href").map(|href| href.to_string()))
                    .collect()
            }).await;

            hrefs.append(&mut vec![format!("/favicon.svg"), format!("/favicon.ico")]);

            for href in hrefs {
                let absolute = Url::parse(&href).unwrap_or(url.join(&href).unwrap());
                let Ok(response) = client.get(absolute).send().await else {
                    continue;
                };
                let Ok(bytes) = response.bytes().await else {
                    continue;
                };

                if let Ok(format) = image::guess_format(&bytes) {
                    if format == ImageFormat::Png {
                        let _ = fs::write(&cache, &bytes).await;
                    } else {
                        let Ok(img) = image::load_from_memory_with_format(&bytes, format) else {
                            continue;
                        };
                        let _ = img.save_with_format(&cache, ImageFormat::Png);
                    }
                    if let Ok(dimensions) = image::image_dimensions(&cache) {
                        if dimensions.0 > 50 {
                            return Ok(cache);
                        }
                    }
                } else {
                    let tree = resvg::usvg::Tree::from_data(
                        &bytes,
                        &resvg::usvg::Options::default(),
                        &resvg::usvg::fontdb::Database::default(),
                    )?;
                    let size = tree.size();
                    let width = 64;
                    let height = 64;
                    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
                        .expect("failed to create pixmap");
                    let mut pixmap_mut = pixmap.as_mut();
                    resvg::render(
                        &tree,
                        resvg::usvg::Transform::from_scale(
                            width as f32 / size.width(),
                            height as f32 / size.height(),
                        ),
                        &mut pixmap_mut,
                    );
                    if pixmap.save_png(&cache).is_ok() {
                        return Ok(cache);
                    }
                }
            }
        }

        Err(anyhow!("No favicon found for {}", url))
    }
    pub fn new(url: impl ToString, fallback: Icon, cx: &mut WindowContext) -> View<Self> {
        cx.new_view(|_| Self {
            url: url.to_string(),
            fallback,
            path: None,
            init: OnceCell::new(),
        })
    }
    pub fn init(&mut self, cx: &mut ViewContext<Self>) {
        self.init.get_or_init(|| {
            let Ok(url) = Url::parse(&self.url) else {
                return true;
            };
            if url.cannot_be_a_base() || !url.scheme().starts_with("http") {
                return true;
            };

            let Some(host) = url.host_str() else {
                return true;
            };

            let cache = paths().cache.join("favicons").join(format!("{}.png", host));

            if let Ok(exists) = cache.try_exists() {
                if exists {
                    self.path = Some(cache);
                    cx.notify();
                    return true;
                }
            }

            cx.spawn(|view, mut cx| async move {
                // TODO: For some reason, if lots of favicons are requested at once, only the first x will be fetched.
                let result = spawn(Self::fetch_favicon(url, cache)).await;
                if let Ok(path) = result {
                    let _ = view.update(&mut cx, |this, cx| {
                        this.path = Some(path);
                        cx.notify();
                    });
                } else {
                    debug!("Failed to fetch favicon: {:?}", result.err());
                }
            })
            .detach();
            true
        });
    }
}

impl Render for Favicon {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.init(cx);
        if let Some(path) = self.path.clone() {
            Img::list_file(path)
        } else {
            Img::list_icon(self.fallback.clone(), None)
        }
    }
}
