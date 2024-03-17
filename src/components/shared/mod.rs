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
    cell::OnceCell,
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use anyhow::anyhow;
use async_std::task::{spawn, spawn_blocking, JoinHandle};
use futures::future::Shared;
use futures::FutureExt;
use gpui::*;
use log::{debug, error};
use parking_lot::Mutex;
use scraper::{Html, Selector};
use url::Url;

pub use icon::Icon;

use crate::theme::Theme;

mod icon;

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
    None,
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

impl Default for Img {
    fn default() -> Self {
        Self {
            src: ImgSource::None,
            mask: ImgMask::None,
            size: ImgSize::MD,
        }
    }
}

impl Img {
    pub fn icon(mut self, icon: Icon) -> Self {
        self.src = ImgSource::Icon { icon, color: None };
        self.mask = ImgMask::Rounded;
        self
    }
    pub fn icon_color(mut self, color: Hsla) -> Self {
        let icon = match self.src {
            ImgSource::Icon { icon, color: _ } => icon,
            _ => {
                return self;
            }
        };
        self.src = ImgSource::Icon {
            icon,
            color: Some(color),
        };
        self
    }
    pub fn dot(mut self, color: Hsla) -> Self {
        self.src = ImgSource::Dot(color);
        self
    }
    pub fn favicon(mut self, url: impl ToString, fallback: Icon, cx: &mut WindowContext) -> Self {
        let favicon = Favicon::new(&self, url, fallback, cx);
        self.src = ImgSource::Favicon(favicon);
        self
    }
    pub fn file(mut self, src: PathBuf) -> Self {
        self.src = ImgSource::Base(ImageSource::File(Arc::new(src)));
        self
    }
    pub fn url(mut self, src: impl ToString) -> Self {
        self.src = ImgSource::Base(ImageSource::Uri(SharedUri::from(src.to_string())));
        self
    }
    pub fn mask(mut self, mask: ImgMask) -> Self {
        self.mask = mask;
        self
    }
    pub fn size(mut self, size: ImgSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for Img {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        if let ImgSource::Favicon(favicon) = &self.src {
            return favicon.clone().into_any_element();
        }
        let theme = cx.global::<Theme>();
        let el = div().flex().items_center().justify_center();
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
                let img = img(src).object_fit(ObjectFit::Cover).size_full();
                let img = match self.mask {
                    ImgMask::Circle => {
                        el = el.p_0p5();
                        img.rounded_full().overflow_hidden().bg(theme.surface0)
                    }
                    ImgMask::Rounded => {
                        el = el.p_0p5();
                        img.rounded_md().overflow_hidden().bg(theme.surface0)
                    }
                    ImgMask::None => img,
                };
                img.into_any_element()
            }
            ImgSource::Dot(color) => div().rounded_full().bg(color).size_1_2().into_any_element(),
            ImgSource::Favicon(_) => unreachable!(),
            ImgSource::None => div().into_any_element(),
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

type FetchFaviconTask = Shared<JoinHandle<Result<SharedUri, Arc<anyhow::Error>>>>;

static FAVICONS: OnceLock<Arc<Mutex<HashMap<String, FetchFaviconTask>>>> = OnceLock::new();

#[derive(Clone)]
pub struct Favicon {
    img: Img,
    fallback: Icon,
    url: String,
    task: OnceCell<FetchFaviconTask>,
}

impl Favicon {
    async fn find_favicon(url: impl ToString) -> Result<SharedUri, anyhow::Error> {
        let url = Url::parse(&url.to_string())?;
        if url.cannot_be_a_base() || !url.scheme().starts_with("http") {
            return Err(anyhow!("Invalid URL"));
        };
        let base_url = Url::parse(&format!("{}://{}", url.scheme(), url.host_str().unwrap()))?;
        let mut targets = vec![base_url.clone()];
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
            .user_agent("http_client (loungy.app)")
            .build()?;
        for target in targets {
            let Ok(response) = client.get(target.clone()).send().await else {
                continue;
            };
            let url = response.url().clone();
            let Ok(html) = response.text().await else {
                continue;
            };

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
                let Ok(response) = client.get(absolute.to_string()).send().await else {
                    continue;
                };
                if response.status().is_success() {
                    return Ok(absolute.to_string().into());
                };
            }
        }

        Err(anyhow!("No favicon found for {}", url))
    }
    pub fn new(
        img: &Img,
        url: impl ToString,
        fallback: Icon,
        cx: &mut WindowContext,
    ) -> View<Self> {
        let url = url.to_string();
        cx.new_view(|_cx| Self {
            img: img.clone(),
            fallback,
            url: url.to_string(),
            task: OnceCell::new(),
        })
    }
}

impl Render for Favicon {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        if let Some(task) = self
            .task
            .get_or_init(|| {
                FAVICONS
                    .get_or_init(Default::default)
                    .lock()
                    .entry(self.url.clone())
                    .or_insert_with(|| {
                        let url = self.url.clone();
                        spawn(async move {
                            Self::find_favicon(url).await.map_err(|err| {
                                let error = Arc::new(err);
                                debug!("{}", error);
                                error
                            })
                        })
                        .shared()
                    })
                    .clone()
            })
            .clone()
            .now_or_never()
            .and_then(|result| result.ok())
        {
            self.img.clone().url(task)
        } else {
            self.img.clone().icon(self.fallback.clone())
        }
    }
}
