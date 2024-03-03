use std::{
    cmp::Reverse,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use arboard::Clipboard;
use async_std::task::sleep;
use bonsaidb::core::schema::SerializedCollection;
use gpui::*;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{AsyncListItems, Item, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    paths::paths,
    query::TextInputWeak,
    state::{ActionsModel, StateItem, StateModel, StateViewBuilder},
    swift,
    theme::Theme,
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
                                .downcast_ref::<ClipboardEntry>()
                                .unwrap()
                                .timestamp,
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

#[derive(Clone)]
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

#[derive(Clone)]
struct ClipboardEntry {
    timestamp: Instant,
    application: String,
    application_icon: Option<Img>,
    kind: ClipboardKind,
    state: ListState,
}

impl ClipboardEntry {
    fn new(kind: ClipboardKind) -> Self {
        #[cfg(target_os = "macos")]
        let (application, icon) = {
            let app = unsafe { swift::get_frontmost_application_data() };
            if let Some(app) = app {
                let mut icon_path = paths().cache.clone();
                icon_path.push(format!("{}.png", app.id.to_string()));
                (app.name.to_string(), Some(Img::list_file(icon_path)))
            } else {
                ("Unknown".to_string(), None)
            }
        };
        #[cfg(not(target_os = "macos"))]
        let (application, icon) = ("Unknown".to_string(), None);
        Self {
            timestamp: Instant::now(),
            application,
            application_icon: icon,
            kind: kind.clone(),
            state: ListState::new(1, ListAlignment::Top, Pixels(100.0), move |_, _| match kind
                .clone()
            {
                ClipboardKind::Text { text, .. } => {
                    div().w_full().child(text.clone()).into_any_element()
                }
                _ => div().child("Unimplemented").into_any_element(),
            }),
        }
    }
}

impl Render for ClipboardEntry {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        match self.kind.clone() {
            ClipboardKind::Text {
                characters, words, ..
            } => {
                let table = vec![
                    (
                        "Application".to_string(),
                        div()
                            .flex()
                            .items_center()
                            .child(if let Some(icon) = self.application_icon.clone() {
                                div().child(icon).mr_1()
                            } else {
                                div()
                            })
                            .child(self.application.clone())
                            .into_any_element(),
                    ),
                    ("Content Type".to_string(), "Text".into_any_element()),
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

impl StateViewBuilder for ClipboardEntry {
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

pub struct ClipboardCommandBuilder;

impl RootCommandBuilder for ClipboardCommandBuilder {
    fn build(&self, cx: &mut WindowContext) -> RootCommand {
        let view = cx.new_view(|cx| {
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
                            let entry = ClipboardEntry::new(ClipboardKind::Text {
                                characters: text.chars().count() as u64,
                                words: text.split_whitespace().count() as u64,
                                text: text.clone(),
                            });
                            let item = Item::new(
                                hash,
                                vec![text.clone()],
                                cx.new_view(|_| {
                                    ListItem::new(
                                        Some(Img::list_icon(Icon::File, None)),
                                        {
                                            let mut text = text.trim().replace("\n", " ");
                                            if text.len() > 30 {
                                                text.truncate(30);
                                                text.push_str("...");
                                            }
                                            text
                                        },
                                        None,
                                        vec![],
                                    )
                                })
                                .unwrap()
                                .into(),
                                Some((
                                    0.66,
                                    Box::new({
                                        let entry = entry.clone();
                                        move |cx| StateItem::init(entry.clone(), false, cx)
                                    }),
                                )),
                                vec![],
                                None,
                                Some(Box::new(entry)),
                                None,
                            );
                            let _ = view.update(&mut cx, |view: &mut AsyncListItems, cx| {
                                view.push("text".to_string(), item, cx)
                            });
                        }
                    }
                    // if let Ok(image) = clipboard.get_image() {

                    // }
                    sleep(Duration::from_micros(250)).await;
                }
            })
            .detach();
            AsyncListItems::new()
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
