use std::{
    cmp::Reverse,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::{mpsc::Receiver, OnceLock},
    time::{Duration, Instant},
};

use arboard::Clipboard;
use async_std::task::sleep;
use bonsaidb::{
    core::{
        connection::LowLevelConnection,
        schema::{Collection, SerializedCollection},
    },
    local::Database,
};
use gpui::*;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use swift_rs::SRString;
use time::{format_description, Date, OffsetDateTime};

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{AsyncListItems, Item, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    db::Db,
    paths::paths,
    query::TextInputWeak,
    state::{Action, ActionsModel, StateItem, StateModel, StateViewBuilder},
    swift,
    theme::Theme,
    window::Window,
};

#[derive(Clone)]
pub struct ClipboardListBuilder {
    view: View<AsyncListItems>,
}

impl StateViewBuilder for ClipboardListBuilder {
    fn build(
        &self,
        query: &TextInputWeak,
        actions: &ActionsModel,
        update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search your clipboard history...", cx);

        AsyncListItems::loader(&self.view, &actions, cx);
        let view = self.view.clone();
        ListBuilder::new()
            .build(
                query,
                &actions,
                move |_list, _, cx| {
                    let items = view.read(cx).items.clone();
                    let mut items: Vec<Item> = items.values().flatten().cloned().collect();
                    items.sort_by_key(|item| {
                        Reverse(
                            item.meta
                                .value()
                                .downcast_ref::<ClipboardListItem>()
                                .unwrap()
                                .copied_last,
                        )
                    });
                    return Ok(Some(items));
                },
                None,
                None,
                update_receiver,
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
    Image {
        width: u64,
        height: u64,
        content: Vec<u8>,
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
    Image,
}

impl Into<ClipboardListItemKind> for ClipboardKind {
    fn into(self) -> ClipboardListItemKind {
        match self {
            ClipboardKind::Text { .. } => ClipboardListItemKind::Text,
            ClipboardKind::Image { .. } => ClipboardListItemKind::Image,
        }
    }
}

impl Into<String> for ClipboardListItemKind {
    fn into(self) -> String {
        match self {
            ClipboardListItemKind::Text => "Text".to_string(),
            ClipboardListItemKind::Image => "Image".to_string(),
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
        #[cfg(target_os = "macos")]
        let (application, icon_path) = {
            let app = unsafe { swift::get_frontmost_application_data() };
            if let Some(app) = app {
                let mut icon_path = paths().cache.clone();
                icon_path.push(format!("{}.png", app.id.to_string()));
                (app.name.to_string(), Some(icon_path))
            } else {
                ("Unknown".to_string(), None)
            }
        };
        #[cfg(not(target_os = "macos"))]
        let (application, icon_path) = ("Unknown".to_string(), None);

        let item = Self {
            id: id.clone(),
            title: title.to_string(),
            copied_last: OffsetDateTime::now_utc(),
            copied_first: OffsetDateTime::now_utc(),
            copy_count: 1,
            kind: kind.clone().into(),
        };
        let _ = item.clone().push_into(db_items());
        let detail = ClipboardDetail {
            id: id,
            application,
            application_icon: icon_path,
            kind,
        };
        let _ = detail.push_into(db_detail());

        item
    }
    fn get_item(&self, cx: &mut WindowContext) -> Item {
        Item::new(
            self.id,
            vec![self.title.clone()],
            cx.new_view(|_| {
                ListItem::new(
                    Some(Img::list_icon(Icon::File, None)),
                    self.title.clone(),
                    None,
                    vec![],
                )
            })
            .into(),
            Some((
                0.66,
                Box::new({
                    let id = self.id.clone();
                    move |cx| StateItem::init(ClipboardPreview::init(id, cx), false, cx)
                }),
            )),
            vec![
                Action::new(
                    Img::list_icon(Icon::ClipboardPaste, None),
                    "Paste",
                    None,
                    {
                        let id = self.id.clone();
                        move |_, cx| {
                            let detail = ClipboardDetail::get(&id, db_detail()).unwrap().unwrap();
                            match detail.contents.kind.clone() {
                                ClipboardKind::Text { text, .. } => {
                                    swift::close_and_paste(text.as_str(), false, cx);
                                }
                                _ => {}
                            }
                        }
                    },
                    false,
                ),
                Action::new(
                    Img::list_icon(Icon::ClipboardPaste, None),
                    "Paste Formatted",
                    None,
                    {
                        let id = self.id.clone();
                        move |_, cx| {
                            let detail = ClipboardDetail::get(&id, db_detail()).unwrap().unwrap();
                            match detail.contents.kind.clone() {
                                ClipboardKind::Text { text, .. } => {
                                    swift::close_and_paste(text.as_str(), true, cx);
                                }
                                _ => {}
                            }
                        }
                    },
                    false,
                ),
                Action::new(
                    Img::list_icon(Icon::Trash, None),
                    "Delete",
                    None,
                    {
                        let id = self.id.clone();
                        move |_, cx| {
                            let _ = ClipboardListItem::get(&id, db_items())
                                .unwrap()
                                .unwrap()
                                .delete(db_items());
                            let _ = ClipboardDetail::get(&id, db_detail())
                                .unwrap()
                                .unwrap()
                                .delete(db_detail());
                        }
                    },
                    false,
                ),
            ],
            None,
            Some(Box::new(self.clone())),
            None,
        )
    }
}

#[derive(Clone)]
struct ClipboardPreview {
    id: u64,
    item: ClipboardListItem,
    detail: ClipboardDetail,
    state: ListState,
}

impl ClipboardPreview {
    fn init(id: u64, cx: &mut WindowContext) -> Self {
        let item = ClipboardListItem::get(&id, db_items()).unwrap().unwrap();
        let detail = ClipboardDetail::get(&id, db_detail()).unwrap().unwrap();
        Self {
            id,
            item: item.contents,
            detail: detail.clone().contents,
            state: ListState::new(
                1,
                ListAlignment::Top,
                Pixels(100.0),
                move |_, _| match detail.contents.kind.clone() {
                    ClipboardKind::Text { text, .. } => {
                        div().w_full().child(text.clone()).into_any_element()
                    }
                    _ => div().child("Unimplemented").into_any_element(),
                },
            ),
        }
    }
}

impl Render for ClipboardPreview {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        match self.detail.kind.clone() {
            ClipboardKind::Text {
                characters, words, ..
            } => {
                let table = vec![
                    (
                        "Application".to_string(),
                        div()
                            .flex()
                            .items_center()
                            .child(if let Some(icon) = self.detail.application_icon.clone() {
                                div().child(Img::list_file(icon)).mr_1()
                            } else {
                                div()
                            })
                            .child(self.detail.application.clone())
                            .into_any_element(),
                    ),
                    (
                        "Last Copied".to_string(),
                        self.item
                            .copied_last
                            .format(
                                &format_description::parse(
                                    "[year]/[month]/[day] [hour]:[minute]:[second]",
                                )
                                .unwrap(),
                            )
                            .unwrap()
                            .into_any_element(),
                    ),
                    (
                        "First Copied".to_string(),
                        self.item
                            .copied_first
                            .format(
                                &format_description::parse(
                                    "[year]/[month]/[day] [hour]:[minute]:[second]",
                                )
                                .unwrap(),
                            )
                            .unwrap()
                            .into_any_element(),
                    ),
                    (
                        "Times Copied".to_string(),
                        self.item.copy_count.to_string().into_any_element(),
                    ),
                    ("Content Type".to_string(), {
                        let kind: String = self.item.kind.clone().into();
                        kind.into_any_element()
                    }),
                    (
                        "Characters".to_string(),
                        characters.to_string().into_any_element(),
                    ),
                    ("Words".to_string(), words.to_string().into_any_element()),
                ];
                div()
                    .ml_2()
                    .pl_2()
                    .border_l_1()
                    .border_color(theme.surface0)
                    .h_full()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .flex_1()
                            .p_2()
                            .text_xs()
                            .font(theme.font_mono.clone())
                            .child(list(self.state.clone()).size_full()),
                    )
                    .child(
                        div()
                            .border_t_1()
                            .border_color(theme.surface0)
                            .mt_auto()
                            .text_sm()
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
                                            .child(value)
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                    )
            }
            _ => div().child("Unimplemented"),
        }
    }
}

impl StateViewBuilder for ClipboardPreview {
    fn build(
        &self,
        _query: &TextInputWeak,
        _actions: &ActionsModel,
        _update_receiver: Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        cx.new_view(|_| self.clone()).into()
    }
}

pub(super) fn db_items() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(|| Db::init_collection::<ClipboardListItem>())
}

pub(super) fn db_detail() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(|| Db::init_collection::<ClipboardDetail>())
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
                loop {
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
                                ClipboardListItem::new(
                                    hash.clone(),
                                    {
                                        let mut text = text.trim().replace("\n", " ");
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
                            };
                            let _ = cx.update_window(cx.window_handle(), |_, cx| {
                                let _ = view.update(cx, |view: &mut AsyncListItems, cx| {
                                    let item = entry.get_item(cx);
                                    view.push(entry.kind.into(), item, cx);
                                });
                            });
                        }
                    }
                    // if let Ok(image) = clipboard.get_image() {

                    // }
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
