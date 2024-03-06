use std::{cmp::Reverse, collections::HashMap};

use bonsaidb::core::schema::SerializedCollection;
use futures::StreamExt;
use gpui::*;
use matrix_sdk::{
    ruma::{events::room::message::OriginalSyncRoomMessageEvent, OwnedRoomId},
    Client, SlidingSync,
};

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{AsyncListItems, Item, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img, ImgMask},
    },
    state::{Action, Shortcut, StateItem, StateModel, StateViewBuilder, StateViewContext},
};

use super::{
    account::AccountCreationBuilder,
    chat::ChatRoom,
    client::{db, Session},
    compose::{Compose, ComposeKind},
    mxc::mxc_to_http,
};

#[derive(Clone)]
pub(super) struct RoomUpdate {
    pub client: Client,
    pub sliding_sync: SlidingSync,
}
pub(super) enum RoomUpdateEvent {
    Update(OwnedRoomId),
}
impl EventEmitter<RoomUpdateEvent> for RoomUpdate {}

#[derive(Clone)]
struct RoomList {
    view: View<AsyncListItems>,
}

impl StateViewBuilder for RoomList {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context.query.set_placeholder("Search your rooms...", cx);
        if let Ok(accounts) = Session::all(db()).query() {
            if accounts.len() > 1 {
                let mut options = vec![("".to_string(), "Show All".to_string())];
                for account in accounts {
                    let id = account.contents.id.clone();
                    options.push((id.clone(), id));
                }
                context.actions.set_dropdown("", options, cx);
            }
        }

        AsyncListItems::loader(&self.view, &context.actions, cx);
        let view = self.view.clone();
        ListBuilder::new()
            .build(
                move |list, _, cx| {
                    let account = list.actions.get_dropdown_value(cx);
                    let items = view.read(cx).items.clone();
                    let mut items = if account.is_empty() {
                        items.values().flatten().cloned().collect()
                    } else {
                        items.get(&account).cloned().unwrap_or_default()
                    };
                    items.sort_unstable_by_key(|item| {
                        let timestamp: u64 = item.get_meta();
                        Reverse(timestamp)
                    });
                    Ok(Some(items))
                },
                None,
                context,
                cx,
            )
            .into()
    }
}

pub struct MatrixCommandBuilder;

async fn sync(
    session: Session,
    view: WeakView<AsyncListItems>,
    mut cx: AsyncWindowContext,
) -> Result<()> {
    loop {
        let (client, ss) = session.load().await?;
        let sync = ss.sync();
        let mut sync_stream = Box::pin(sync);
        let server = client.homeserver();
        let model = cx
            .new_model(|_| RoomUpdate {
                client: client.clone(),
                sliding_sync: ss.clone(),
            })
            .unwrap();

        let mut previews = HashMap::<OwnedRoomId, ChatRoom>::new();
        while let Some(Ok(response)) = sync_stream.next().await {
            if response.rooms.is_empty() {
                continue;
            }

            let list: Vec<Item> = ss
                .get_all_rooms()
                .await
                .iter()
                .map(|room| {
                    let mut queue = room.timeline_queue().into_iter();
                    let timestamp: u64 = loop {
                        let Some(ev) = queue.next_back() else {
                            break 0;
                        };
                        match ev.event.deserialize_as::<OriginalSyncRoomMessageEvent>() {
                            Ok(m) => {
                                break m.origin_server_ts.as_secs().into();
                            }
                            Err(_) => {
                                continue;
                            }
                        }
                    };
                    let name = room.name().unwrap_or("".to_string());
                    let mut img = match room.avatar_url() {
                        Some(source) => Img::list_url(mxc_to_http(server.clone(), source, true)),
                        None => match room.is_dm() {
                            Some(true) => Img::list_icon(Icon::User, None),
                            _ => Img::list_icon(Icon::Users, None),
                        },
                    };

                    img.mask = ImgMask::Circle;

                    let room_id = room.room_id().to_owned();
                    let _ = model.update(&mut cx, |_, cx| {
                        cx.emit(RoomUpdateEvent::Update(room_id.clone()));
                    });
                    let preview = if let Some(preview) = previews.get(&room_id) {
                        preview.clone()
                    } else {
                        let preview = ChatRoom {
                            room_id: room_id.clone(),
                            updates: model.clone(),
                        };
                        previews.insert(room_id.clone(), preview.clone());
                        preview
                    };

                    ItemBuilder::new(
                        room_id.clone(),
                        ListItem::new(Some(img), name.clone(), None, vec![]),
                    )
                    .keywords(vec![name.clone()])
                    .actions(vec![
                        Action::new(
                            Img::list_icon(Icon::MessageCircle, None),
                            "Write",
                            None,
                            {
                                let client = client.clone();
                                let room_id = room_id.clone();
                                move |_, cx| {
                                    let item = StateItem::init(
                                        Compose::new(
                                            client.clone(),
                                            room_id.clone(),
                                            ComposeKind::Message,
                                        ),
                                        false,
                                        cx,
                                    );
                                    StateModel::update(|this, cx| this.push_item(item, cx), cx);
                                }
                            },
                            false,
                        ),
                        Action::new(
                            Img::list_icon(Icon::Search, None),
                            "Search",
                            Some(Shortcut::cmd("/")),
                            |actions, cx| {
                                StateModel::update(
                                    |this, cx| this.push_item(actions.active.clone().unwrap(), cx),
                                    cx,
                                );
                            },
                            false,
                        ),
                    ])
                    .preview(0.66, move |cx| StateItem::init(preview.clone(), false, cx))
                    .meta(timestamp)
                    .build()
                })
                .collect();

            let id = session.id.clone();
            if let Some(view) = view.upgrade() {
                view.update(&mut cx, |view, cx| {
                    view.update(id, list, cx);
                })?;
            } else {
                break;
            }
        }
    }
}

impl RootCommandBuilder for MatrixCommandBuilder {
    fn build(&self, cx: &mut WindowContext) -> RootCommand {
        let view = cx.new_view(|cx| {
            let db = db();
            let sessions = Session::all(db).query().unwrap_or_default();
            for session in sessions {
                cx.spawn(move |view, cx| async { sync(session.contents, view, cx).await })
                    .detach();
            }

            AsyncListItems::new()
        });
        RootCommand::new(
            "matrix",
            "Search Rooms",
            "Matrix",
            Icon::MessageCircle,
            vec!["Chat", "Messages"],
            None,
            Box::new(move |_, cx| {
                let view = view.clone();
                let sessions = Session::all(db());
                if sessions.count().unwrap_or_default() == 0 {
                    StateModel::update(|this, cx| this.push(AccountCreationBuilder, cx), cx);
                } else {
                    StateModel::update(|this, cx| this.push(RoomList { view }, cx), cx);
                };
            }),
        )
    }
}
