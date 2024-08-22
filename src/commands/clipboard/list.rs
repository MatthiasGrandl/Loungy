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
    cmp::Reverse,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::{Arc, OnceLock},
    thread,
    time::{Duration, Instant},
};

use arboard::Clipboard;
use bonsaidb::{
    core::schema::{Collection, SerializedCollection},
    local::Database,
};
use gpui::*;
use image::{DynamicImage, ImageBuffer};
use jiff::{Span, Timestamp, ToSpan};
use log::error;
use serde::{Deserialize, Serialize};

use url::Url;

use crate::{
    command,
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{AsyncListItems, Item, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img, ImgMask, ImgSize, ObjectFit},
    },
    date::format_date,
    db::Db,
    paths::paths,
    platform::{
        clipboard, close_and_paste, close_and_paste_file, get_frontmost_application_data, ocr,
        AppData, ClipboardWatcher,
    },
    state::{
        Action, CommandTrait, Shortcut, StateItem, StateModel, StateViewBuilder, StateViewContext,
    },
    theme::Theme,
};

#[derive(Clone)]
pub struct ClipboardListBuilder {
    view: View<AsyncListItems>,
}
command!(ClipboardListBuilder);
impl StateViewBuilder for ClipboardListBuilder {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context
            .query
            .set_placeholder("Search your clipboard history...", cx);

        context.actions.update_global(
            vec![Action::new(
                Img::default().icon(Icon::Trash),
                "Delete All",
                None,
                {
                    let view = self.view.clone();
                    move |actions, cx| {
                        if let Err(err) =
                            ClipboardListItem::prune(ToSpan::seconds(0), view.downgrade(), cx)
                        {
                            error!("Failed to prune clipboard: {:?}", err);
                            actions
                                .toast
                                .error("Failed to delete clipboard entries", cx);
                        } else {
                            actions
                                .toast
                                .success("Successfully deleted clipboard entries", cx);
                        }
                    }
                },
                false,
            )],
            cx,
        );

        context.actions.set_dropdown(
            "memory",
            vec![
                ("", "All Types"),
                ("Text", "Text Only"),
                ("Link", "Links Only"),
                ("Image", "Images Only"),
            ],
            cx,
        );

        AsyncListItems::loader(&self.view, &context.actions, cx);
        let view = self.view.clone();
        ListBuilder::new()
            .build(
                move |list, _, cx| {
                    let t = list.actions.get_dropdown_value(cx);
                    let items = view.read(cx).items.clone();
                    let mut items: Vec<Item> = if t.is_empty() {
                        items.values().flatten().cloned().collect()
                    } else {
                        items.get(&t).cloned().unwrap_or_default()
                    };

                    items.sort_by_key(|item| Reverse(item.get_meta::<Timestamp>(cx).unwrap()));
                    Ok(Some(items))
                },
                context,
                cx,
            )
            .into()
    }
}

#[derive(Clone, Serialize, Deserialize)]
enum ClipboardKind {
    Text {
        characters: u64,
        words: u64,
        text: String,
    },
    Url {
        url: String,
        characters: u64,
        title: String,
    },
    Image {
        width: u32,
        height: u32,
        thumbnail: PathBuf,
        path: PathBuf,
    },
}

#[derive(Clone, Serialize, Deserialize, Collection)]
#[collection(name = "clipboard.detail")]
struct ClipboardDetail {
    #[natural_id]
    id: u64,
    application: String,
    application_icon: Option<PathBuf>,
    kind: ClipboardKind,
}

#[derive(Clone, Serialize, Deserialize)]
enum ClipboardListItemKind {
    Text,
    Url { url: String },
    Image { thumbnail: PathBuf },
}

impl From<ClipboardKind> for ClipboardListItemKind {
    fn from(val: ClipboardKind) -> Self {
        match val {
            ClipboardKind::Text { .. } => ClipboardListItemKind::Text,
            ClipboardKind::Url { url, .. } => ClipboardListItemKind::Url { url },
            ClipboardKind::Image { thumbnail, .. } => ClipboardListItemKind::Image { thumbnail },
        }
    }
}

impl From<ClipboardListItemKind> for String {
    fn from(val: ClipboardListItemKind) -> Self {
        match val {
            ClipboardListItemKind::Text => "Text".to_string(),
            ClipboardListItemKind::Url { .. } => "Link".to_string(),
            ClipboardListItemKind::Image { .. } => "Image".to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Collection)]
#[collection(name = "clipboard.item")]
struct ClipboardListItem {
    #[natural_id]
    id: u64,
    title: String,
    copied_first: Timestamp,
    copied_last: Timestamp,
    kind: ClipboardListItemKind,
    copy_count: u32,
}

impl ClipboardListItem {
    fn new(id: u64, title: impl ToString, kind: ClipboardKind, app: &Option<AppData>) -> Self {
        let (application, application_icon) = app
            .as_ref()
            .map(|data| (data.name.clone(), Some(data.icon_path.clone())))
            .unwrap_or(("Unknown".to_string(), None));

        let item = Self {
            id,
            title: title.to_string(),
            copied_last: Timestamp::now(),
            copied_first: Timestamp::now(),
            copy_count: 1,
            kind: kind.clone().into(),
        };
        let _ = item.clone().push_into(db_items());
        let detail = ClipboardDetail {
            id,
            application,
            application_icon,
            kind,
        };
        let _ = detail.push_into(db_detail());

        item
    }
    fn get_item(&self, cx: &mut ViewContext<AsyncListItems>) -> Item {
        ItemBuilder::new(
            self.id,
            ListItem::new(
                match self.kind.clone() {
                    ClipboardListItemKind::Image { thumbnail } => Some(
                        Img::default()
                            .file(thumbnail)
                            .object_fit(ObjectFit::Contain),
                    ),
                    ClipboardListItemKind::Url { url } => Some(
                        Img::default()
                            .mask(ImgMask::Rounded)
                            .favicon(url, Icon::Link, cx),
                    ),
                    _ => Some(Img::default().icon(Icon::File)),
                },
                self.title.clone(),
                None,
                vec![],
            ),
        )
        .keywords(vec![self.title.clone()])
        .preview(0.66, {
            let id = self.id;
            move |cx| StateItem::init(ClipboardPreview::init(id, cx), false, cx)
        })
        .actions({
            let mut actions = vec![
                Action::new(
                    Img::default().icon(Icon::ClipboardPaste),
                    "Paste",
                    None,
                    {
                        let id = self.id;
                        move |_, cx| {
                            let detail = ClipboardDetail::get(&id, db_detail()).unwrap().unwrap();
                            let _ = cx.update_window(cx.window_handle(), |_, cx| {
                                match detail.contents.kind.clone() {
                                    ClipboardKind::Text { text, .. }
                                    | ClipboardKind::Url { url: text, .. } => {
                                        close_and_paste(text.as_str(), false, cx);
                                    }
                                    ClipboardKind::Image { path, .. } => {
                                        close_and_paste_file(&path, cx);
                                    }
                                }
                            });
                        }
                    },
                    false,
                ),
                Action::new(
                    Img::default().icon(Icon::Trash),
                    "Delete",
                    None,
                    {
                        let self_clone = self.clone();
                        let view = cx.view().clone();
                        move |actions, cx| {
                            if let Err(err) = self_clone.delete(view.downgrade(), cx) {
                                error!("Failed to delete clipboard entry: {:?}", err);
                                actions.toast.error("Failed to delete clipboard entry", cx);
                            } else {
                                actions
                                    .toast
                                    .success("Successfully deleted clipboard entry", cx);
                            }
                        }
                    },
                    false,
                ),
            ];
            match self.kind.clone() {
                ClipboardListItemKind::Image { thumbnail } => actions.insert(
                    1,
                    Action::new(
                        Img::default().icon(Icon::ScanEye),
                        "Copy Text to Clipboard",
                        Some(Shortcut::new("enter").shift()),
                        {
                            let mut path = thumbnail.clone();
                            path.pop();
                            path = path.join(format!("{}.png", self.id));
                            move |actions, cx| {
                                ocr(&path);
                                actions.toast.success("Copied Text to Clipboard", cx);
                            }
                        },
                        false,
                    ),
                ),
                ClipboardListItemKind::Url { url } => actions.insert(
                    1,
                    Action::new(
                        Img::default().icon(Icon::ArrowUpRightFromSquare),
                        "Open",
                        Some(Shortcut::new("enter").shift()),
                        {
                            let url = url.clone();
                            move |actions, cx| {
                                cx.open_url(&url.clone());
                                actions.toast.floating(
                                    "Opened in browser",
                                    Some(Icon::ArrowUpRightFromSquare),
                                    cx,
                                )
                            }
                        },
                        false,
                    ),
                ),
                _ => {}
            }
            actions
        })
        .meta(cx.new_model(|_| self.copied_last).into_any())
        .build()
    }
    fn delete(&self, view: WeakView<AsyncListItems>, cx: &mut WindowContext) -> anyhow::Result<()> {
        let _ = view.update(cx, |view, cx| {
            view.remove(self.kind.clone().into(), self.id, cx);
        });

        if let Some(item) = ClipboardDetail::get(&self.id, db_detail())? {
            item.delete(db_detail())?;
        };
        if let Some(item) = Self::get(&self.id, db_items())? {
            item.delete(db_items())?;
        };
        if let ClipboardListItemKind::Image { thumbnail } = self.kind.clone() {
            let mut path = thumbnail.clone();
            path.pop();
            let _ = std::fs::remove_file(thumbnail);
            let _ = std::fs::remove_file(path.join(format!("{}.png", self.id)));
        }
        Ok(())
    }
    fn prune(
        age: Span,
        view: WeakView<AsyncListItems>,
        cx: &mut WindowContext,
    ) -> anyhow::Result<()> {
        let items = Self::all(db_items()).query()?;
        for item in items {
            if item.contents.copied_last < Timestamp::now().checked_sub(age).unwrap() {
                let _ = item.contents.delete(view.clone(), cx);
            }
        }
        Ok(())
    }
}

#[derive(Clone)]

struct ClipboardPreview {
    id: u64,
    item: ClipboardListItem,
    detail: ClipboardDetail,
    bounds: Model<Bounds<Pixels>>,
    state: ListState,
}

impl ClipboardPreview {
    fn init(id: u64, cx: &mut WindowContext) -> Self {
        let item = ClipboardListItem::get(&id, db_items())
            .unwrap()
            .unwrap()
            .contents;
        let detail = ClipboardDetail::get(&id, db_detail())
            .unwrap()
            .unwrap()
            .contents;

        let bounds = cx.new_model(|_| Bounds::default());

        Self {
            id,
            item,
            detail: detail.clone(),
            bounds: bounds.clone(),
            state: ListState::new(
                1,
                ListAlignment::Top,
                Pixels(100.0),
                move |_, cx| match detail.kind.clone() {
                    ClipboardKind::Text { text, .. } | ClipboardKind::Url { url: text, .. } => {
                        div().p_2().w_full().child(text.clone()).into_any_element()
                    }
                    ClipboardKind::Image {
                        width,
                        height,
                        path,
                        ..
                    } => {
                        let bounds = bounds.read(cx);
                        let (mut w, mut h) = if height < width {
                            (
                                bounds.size.width.0,
                                bounds.size.width.0 * height as f32 / width as f32,
                            )
                        } else {
                            (
                                bounds.size.width.0 * width as f32 / height as f32,
                                bounds.size.width.0,
                            )
                        };
                        if w > bounds.size.width.0 {
                            h *= bounds.size.width.0 / w;
                            w = bounds.size.width.0;
                        }
                        if h > bounds.size.height.0 {
                            w *= bounds.size.height.0 / h;
                            h = bounds.size.height.0;
                        }
                        let ml = (bounds.size.width.0 - w) / 2.0;
                        let mt = (bounds.size.height.0 - h) / 2.0;

                        div()
                            .child(
                                img(ImageSource::File(Arc::new(path.clone())))
                                    .w(Pixels(w))
                                    .h(Pixels(h)),
                            )
                            .pl(Pixels(ml))
                            .pt(Pixels(mt))
                            .size_full()
                            .into_any_element()
                    }
                },
            ),
        }
    }
}

impl Render for ClipboardPreview {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let mut table = vec![
            (
                "Application".to_string(),
                div()
                    .flex()
                    .items_center()
                    .child(if let Some(icon) = self.detail.application_icon.clone() {
                        div()
                            .child(Img::default().file(icon).size(ImgSize::XS))
                            .mr_1()
                    } else {
                        div()
                    })
                    .child(self.detail.application.clone())
                    .into_any_element(),
            ),
            ("Content Type".to_string(), {
                let kind: String = self.item.kind.clone().into();
                kind.into_any_element()
            }),
        ];
        let ts = format_date(self.item.copied_last, cx).into_any_element();
        table.append(&mut if self.item.copy_count > 1 {
            vec![
                ("Last Copied".to_string(), ts),
                (
                    "First Copied".to_string(),
                    format_date(self.item.copied_first, cx).into_any_element(),
                ),
                (
                    "Times Copied".to_string(),
                    self.item.copy_count.to_string().into_any_element(),
                ),
            ]
        } else {
            vec![("Copied".to_string(), ts)]
        });
        match &self.detail.kind {
            ClipboardKind::Text {
                characters, words, ..
            } => {
                table.push((
                    "Characters".to_string(),
                    characters.to_string().into_any_element(),
                ));
                table.push(("Words".to_string(), words.to_string().into_any_element()));
            }
            ClipboardKind::Url {
                characters, title, ..
            } => {
                table.push((
                    "Characters".to_string(),
                    characters.to_string().into_any_element(),
                ));
                if !title.is_empty() {
                    table.push(("Title".to_string(), title.clone().into_any_element()));
                }
            }
            ClipboardKind::Image { width, height, .. } => {
                table.push((
                    "Dimensions".to_string(),
                    format!("{}x{}", width, height).into_any_element(),
                ));
            }
        }
        div()
            .ml_2()
            .pl_2()
            .border_l_1()
            .border_color(theme.surface0)
            .h_full()
            .flex()
            .flex_col()
            .justify_between()
            .text_xs()
            .child(
                div().flex_1().font_family(theme.font_mono.clone()).child(
                    canvas(
                        {
                            let b = self.bounds.clone();
                            let s = self.state.clone();
                            move |bounds, cx| {
                                b.update(cx, |this, _| {
                                    *this = bounds;
                                });
                                let mut list = list(s).size_full().into_any_element();
                                list.prepaint_as_root(
                                    bounds.origin,
                                    bounds.size.map(AvailableSpace::Definite),
                                    cx,
                                );
                                list
                            }
                        },
                        |_bounds, mut list, cx| list.paint(cx),
                    )
                    .size_full(),
                ),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(theme.surface0)
                    .mt_auto()
                    .p_2()
                    .children(
                        table
                            .into_iter()
                            .map(|(key, value)| {
                                div()
                                    .flex()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(theme.subtext0)
                                            .child(key),
                                    )
                                    .child(div().child(value).font_family(theme.font_mono.clone()))
                            })
                            .collect::<Vec<_>>(),
                    ),
            )
    }
}
command!(ClipboardPreview);

impl StateViewBuilder for ClipboardPreview {
    fn build(&self, _context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        cx.new_view(|_| self.clone()).into()
    }
}

pub(super) fn db_items() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(Db::init_collection::<ClipboardListItem>)
}

pub(super) fn db_detail() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(Db::init_collection::<ClipboardDetail>)
}

pub struct ClipboardCommandBuilder;
command!(ClipboardCommandBuilder);
impl RootCommandBuilder for ClipboardCommandBuilder {
    fn build(&self, cx: &mut WindowContext) -> RootCommand {
        let view = cx.new_view(|cx| {
            let mut list_items = AsyncListItems::new();
            let items = ClipboardListItem::all(db_items())
                .query()
                .unwrap_or_default();
            for item in items {
                let item = item.clone().contents;
                list_items.push(item.kind.clone().into(), item.get_item(cx), cx);
            }
            ClipboardWatcher::init(cx);

            cx.spawn(|view, cx| async move {
                let mut cp = Clipboard::new().unwrap();
                let mut hash: u64 = 0;
                let cache = paths().cache.join("clipboard");
                if !cache.exists() {
                    let _ = std::fs::create_dir_all(&cache);
                }
                let mut now = Instant::now();
                clipboard(
                    |cx| {
                        if Instant::now() - now > Duration::from_secs(3600) {
                            now = Instant::now();
                            // Prune clipboard history every hour, keeping entries for a week
                            let _ = cx.update_window(cx.window_handle(), |_, cx| {
                                let _ = ClipboardListItem::prune(
                                    ToSpan::seconds(60 * 60 * 24 * 7),
                                    view.clone(),
                                    cx,
                                );
                            });
                        }

                        let app = get_frontmost_application_data();
                        let condition = |app: &Option<AppData>, cx: &mut AsyncAppContext| {
                            if !ClipboardWatcher::is_enabled(cx) {
                                ClipboardWatcher::enabled(cx);
                                return false;
                            }

                            // TODO: make this configurable and platform independent
                            if let Some(app) = app {
                                if matches!(
                                    app.id.as_str(),
                                    "com.apple.systempreferences" | "com.apple.keychainaccess"
                                ) {
                                    return false;
                                }
                            }

                            true
                        };
                        if let Ok(text) = cp.get_text() {
                            let mut hasher = DefaultHasher::new();
                            text.hash(&mut hasher);
                            let new_hash = hasher.finish();
                            if new_hash != hash {
                                hash = new_hash;
                                if !condition(&app, cx) {
                                    return;
                                }
                                let entry = if let Ok(Some(mut item)) =
                                    ClipboardListItem::get(&hash, db_items())
                                {
                                    item.contents.copied_last = jiff::Timestamp::now();
                                    item.contents.copy_count += 1;
                                    let _ = item.update(db_items());
                                    item.contents.clone()
                                } else {
                                    let url = Url::parse(&text);
                                    if url.is_ok() && {
                                        let url = url.unwrap();
                                        !url.cannot_be_a_base() && url.scheme().starts_with("http")
                                    } {
                                        ClipboardListItem::new(
                                            hash,
                                            {
                                                let mut text = text.trim().replace('\n', " ");
                                                if text.len() > 25 {
                                                    text.truncate(25);
                                                    text.push_str("...");
                                                }
                                                text
                                            },
                                            ClipboardKind::Url {
                                                characters: text.chars().count() as u64,
                                                url: text,
                                                title: "".to_string(),
                                            },
                                            &app,
                                        )
                                    } else {
                                        ClipboardListItem::new(
                                            hash,
                                            {
                                                let mut text = text.trim().replace('\n', " ");
                                                if text.len() > 25 {
                                                    text.truncate(25);
                                                    text.push_str("...");
                                                }
                                                text
                                            },
                                            ClipboardKind::Text {
                                                characters: text.chars().count() as u64,
                                                words: text.split_whitespace().count() as u64,
                                                text: text.clone(),
                                            },
                                            &app,
                                        )
                                    }
                                };
                                let _ = cx.update(|cx| {
                                    let _ = view.update(cx, |view: &mut AsyncListItems, cx| {
                                        let item = entry.get_item(cx);
                                        view.push(entry.kind.into(), item, cx);
                                    });
                                });
                            }
                        } else if let Ok(image) = cp.get_image() {
                            let mut hasher = DefaultHasher::new();
                            image.bytes.hash(&mut hasher);
                            let new_hash = hasher.finish();
                            if new_hash != hash {
                                hash = new_hash;
                                if !condition(&app, cx) {
                                    return;
                                }
                                let entry = if let Ok(Some(mut item)) =
                                    ClipboardListItem::get(&hash, db_items())
                                {
                                    item.contents.copied_last = Timestamp::now();
                                    item.contents.copy_count += 1;
                                    let _ = item.update(db_items());
                                    item.contents.clone()
                                } else {
                                    let width = image.width.try_into().unwrap();
                                    let height = image.height.try_into().unwrap();
                                    let path = cache.join(format!("{}.png", hash));
                                    let thumbnail = cache.join(format!("{}.thumb.png", hash));
                                    // Spawn a thread to generate thumbnail and saving to filesystem.
                                    {
                                        let path = path.clone();
                                        let thumbnail = thumbnail.clone();
                                        thread::spawn(move || {
                                            let image = DynamicImage::ImageRgba8(
                                                ImageBuffer::from_vec(
                                                    width,
                                                    height,
                                                    image.bytes.to_vec(),
                                                )
                                                .unwrap(),
                                            );
                                            let _ = image.save(&path);
                                            let t = image.thumbnail(64, 64);
                                            let _ = t.save(&thumbnail);
                                        });
                                    }
                                    ClipboardListItem::new(
                                        hash,
                                        format!("Image ({}x{})", width, height),
                                        ClipboardKind::Image {
                                            width,
                                            height,
                                            path,
                                            thumbnail,
                                        },
                                        &app,
                                    )
                                };
                                let _ = cx.update_window(cx.window_handle(), |_, cx| {
                                    let _ = view.update(cx, |view: &mut AsyncListItems, cx| {
                                        let item = entry.get_item(cx);
                                        view.push(entry.kind.into(), item, cx);
                                    });
                                });
                            }
                        }
                    },
                    cx,
                )
                .await;
            })
            .detach();
            list_items
        });

        RootCommand::new(
            "clipboard",
            "Clipboard History",
            "Clipboard",
            Icon::Clipboard,
            Vec::<String>::new(),
            None,
            move |_, cx| {
                let view = view.clone();
                StateModel::update(|this, cx| this.push(ClipboardListBuilder { view }, cx), cx);
            },
        )
    }
}
