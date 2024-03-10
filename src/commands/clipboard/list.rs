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
use async_std::task::sleep;
use bonsaidb::{
    core::schema::{Collection, SerializedCollection},
    local::Database,
};
use gpui::*;
use image::{DynamicImage, ImageBuffer};
use log::error;
use serde::{Deserialize, Serialize};
use time::{format_description, OffsetDateTime};
use tz::TimeZone;
use url::Url;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{AsyncListItems, Item, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img, ImgMask, ImgSize},
    },
    db::Db,
    paths::paths,
    platform::{close_and_paste, close_and_paste_file, get_focused_app_data, get_text_from_image},
    state::{Action, StateItem, StateModel, StateViewBuilder, StateViewContext},
    theme::Theme,
};

#[derive(Clone)]
pub struct ClipboardListBuilder {
    view: View<AsyncListItems>,
}

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
                            ClipboardListItem::prune(Duration::from_secs(0), view.downgrade(), cx)
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

                    items.sort_by_key(|item| Reverse(item.get_meta::<OffsetDateTime>(cx).unwrap()));
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
    #[serde(with = "time::serde::iso8601")]
    copied_first: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    copied_last: OffsetDateTime,
    kind: ClipboardListItemKind,
    copy_count: u32,
}

impl ClipboardListItem {
    fn new(id: u64, title: impl ToString, kind: ClipboardKind) -> Self {
        let (application, application_icon) = get_focused_app_data()
            .map(|data| (data.name, Some(data.icon_path)))
            .unwrap_or(("Unknown".to_string(), None));

        let item = Self {
            id,
            title: title.to_string(),
            copied_last: OffsetDateTime::now_utc(),
            copied_first: OffsetDateTime::now_utc(),
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
                    ClipboardListItemKind::Image { thumbnail } => {
                        Some(Img::default().file(thumbnail))
                    }
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
            if let ClipboardListItemKind::Image { thumbnail } = self.kind.clone() {
                actions.insert(
                    1,
                    Action::new(
                        Img::default().icon(Icon::ScanEye),
                        "Copy Text to Clipboard",
                        None,
                        {
                            let mut path = thumbnail.clone();
                            path.pop();
                            path = path.join(format!("{}.png", self.id));
                            move |actions, cx| {
                                get_text_from_image(&path);
                                actions.toast.success("Copied Text to Clipboard", cx);
                            }
                        },
                        false,
                    ),
                )
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
        age: Duration,
        view: WeakView<AsyncListItems>,
        cx: &mut WindowContext,
    ) -> anyhow::Result<()> {
        let items = Self::all(db_items()).query()?;
        for item in items {
            if item.contents.copied_last < OffsetDateTime::now_utc() - age {
                let _ = item.contents.delete(view.clone(), cx);
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct ClipboardPreview {
    id: u64,
    item: ClipboardListItem,
    detail: ClipboardDetail,
    bounds: Model<Bounds<Pixels>>,
    state: ListState,
    offset: i32,
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
        let offset = TimeZone::local()
            .unwrap()
            .find_current_local_time_type()
            .unwrap()
            .ut_offset();

        Self {
            id,
            item,
            detail: detail.clone(),
            bounds: bounds.clone(),
            offset,
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

fn format_date(date: &OffsetDateTime, offset: i32) -> String {
    let prefix = if date.day() == OffsetDateTime::now_utc().day() {
        "Today"
    } else if date.day()
        == OffsetDateTime::now_utc()
            .saturating_sub(time::Duration::days(1))
            .day()
    {
        "Yesterday"
    } else {
        "[day]. [month repr:short] [year]"
    };
    let format = format!("{}, [hour]:[minute]:[second]", prefix);
    let format = format_description::parse(&format).unwrap();

    date.checked_add(time::Duration::seconds(offset as i64))
        .unwrap()
        .format(&format)
        .unwrap()
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
        let ts = format_date(&self.item.copied_last, self.offset).into_any_element();
        table.append(&mut if self.item.copy_count > 1 {
            vec![
                ("Last Copied".to_string(), ts),
                (
                    "First Copied".to_string(),
                    format_date(&self.item.copied_first, self.offset).into_any_element(),
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
                div().flex_1().font(theme.font_mono.clone()).child(
                    canvas({
                        let b = self.bounds.clone();
                        let s = self.state.clone();
                        move |bounds, cx| {
                            b.update(cx, |this, _| {
                                *this = *bounds;
                            });
                            list(s).size_full().into_any_element().draw(
                                bounds.origin,
                                bounds.size.map(AvailableSpace::Definite),
                                cx,
                            );
                        }
                    })
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
                                    .child(div().child(value).font(theme.font_mono.clone()))
                            })
                            .collect::<Vec<_>>(),
                    ),
            )
    }
}

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
            cx.spawn(|view, mut cx| async move {
                let mut clipboard = Clipboard::new().unwrap();
                let mut hash: u64 = 0;
                let cache = paths().cache.join("clipboard");
                if !cache.exists() {
                    let _ = std::fs::create_dir_all(&cache);
                }
                let mut now = Instant::now();
                loop {
                    if Instant::now() - now > Duration::from_secs(3600) {
                        now = Instant::now();
                        // Prune clipboard history every hour, keeping entries for a week
                        let _ = cx.update_window(cx.window_handle(), |_, cx| {
                            let _ = ClipboardListItem::prune(
                                Duration::from_secs(60 * 60 * 24 * 7),
                                view.clone(),
                                cx,
                            );
                        });
                    }
                    if let Ok(text) = clipboard.get_text() {
                        let mut hasher = DefaultHasher::new();
                        text.hash(&mut hasher);
                        let new_hash = hasher.finish();
                        if new_hash != hash {
                            hash = new_hash;
                            let entry = if let Ok(Some(mut item)) =
                                ClipboardListItem::get(&hash, db_items())
                            {
                                item.contents.copied_last = OffsetDateTime::now_utc();
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
                                    )
                                }
                            };
                            let _ = cx.update_window(cx.window_handle(), |_, cx| {
                                let _ = view.update(cx, |view: &mut AsyncListItems, cx| {
                                    let item = entry.get_item(cx);
                                    view.push(entry.kind.into(), item, cx);
                                });
                            });
                        }
                    } else if let Ok(image) = clipboard.get_image() {
                        let mut hasher = DefaultHasher::new();
                        image.bytes.hash(&mut hasher);
                        let new_hash = hasher.finish();
                        if new_hash != hash {
                            hash = new_hash;
                            let entry = if let Ok(Some(mut item)) =
                                ClipboardListItem::get(&hash, db_items())
                            {
                                item.contents.copied_last = OffsetDateTime::now_utc();
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
                    sleep(Duration::from_secs(1)).await;
                }
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
            Box::new(move |_, cx| {
                let view = view.clone();
                StateModel::update(|this, cx| this.push(ClipboardListBuilder { view }, cx), cx);
            }),
        )
    }
}
