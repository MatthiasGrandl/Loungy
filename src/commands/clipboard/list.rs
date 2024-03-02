use std::{
    borrow::Cow,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::{mpsc::Receiver, OnceLock},
    time::{Duration, Instant},
};

use arboard::Clipboard;
use async_std::{
    channel,
    process::{Command, Output},
    task::sleep,
};
use bonsaidb::{
    core::schema::{Collection, SerializedCollection},
    local::Database,
};
use gpui::*;
use log::*;
use serde::{Deserialize, Serialize};
use swift_rs::SRString;
use url::Url;

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{Accessory, AsyncListItems, Item, ListBuilder, ListItem},
        shared::{Icon, Img},
    },
    db::Db,
    paths::paths,
    query::TextInputWeak,
    state::{Action, ActionsModel, Shortcut, StateModel, StateViewBuilder},
    swift::{autofill, keytap},
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
                move |list, _, cx| {
                    let items = view.read(cx).items.clone();
                    return Ok(Some(items.values().flatten().cloned().collect()));
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
enum ClipboardKind<'a> {
    Text {
        characters: u64,
        words: u64,
        text: Cow<'a, str>,
    },
    Image {
        width: u64,
        height: u64,
        content: Cow<'a, [u8]>,
    },
}

#[derive(Clone)]
struct ClipboardEntry<'a> {
    timestamp: Instant,
    application: String,
    kind: ClipboardKind<'a>,
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
                            let entry = ClipboardEntry {
                                timestamp: Instant::now(),
                                application: "Unknown".to_string(),
                                kind: ClipboardKind::Text {
                                    characters: text.chars().count() as u64,
                                    words: text.split_whitespace().count() as u64,
                                    text: Cow::Owned(text.clone()),
                                },
                            };
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
                                None,
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
