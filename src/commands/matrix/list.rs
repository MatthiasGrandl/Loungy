use std::{cmp::Reverse, collections::HashMap, time::Duration};

use async_compat::CompatExt;
use bonsaidb::core::schema::SerializedCollection;
use futures::StreamExt;
use gpui::*;
use matrix_sdk::{
    ruma::{events::room::message::OriginalSyncRoomMessageEvent, OwnedMxcUri, OwnedRoomId},
    Client, SlidingSync,
};

use crate::{
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{AsyncListItems, Item, List, ListItem},
        shared::{Icon, Img, ImgMask},
    },
    query::TextInputWeak,
    state::{ActionsModel, StateItem, StateModel, StateViewBuilder},
};

use super::{
    account::AccountCreationBuilder,
    chat::ChatRoom,
    client::{db, Session},
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
    fn build(
        &self,
        query: &TextInputWeak,
        actions: &ActionsModel,
        update_receiver: std::sync::mpsc::Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search your rooms...", cx);
        if let Ok(accounts) = Session::all(db()).query() {
            if accounts.len() > 1 {
                let mut options = vec![("".to_string(), "Show All".to_string())];
                for account in accounts {
                    let id = account.contents.id.clone();
                    options.push((id.clone(), id));
                }
                actions.clone().set_dropdown("", options, cx);
            }
        }

        AsyncListItems::loader(&self.view, &actions, cx);
        let view = self.view.clone();
        List::new(
            query,
            &actions,
            move |list, _, cx| {
                let account = list.actions.get_dropdown_value(cx);
                let items = view.read(cx).items.clone();
                let mut items = if account.is_empty() {
                    items.values().flatten().cloned().collect()
                } else {
                    items.get(&account).cloned().unwrap_or_default()
                };
                items.sort_unstable_by_key(|item| {
                    let timestamp = item.meta.value().downcast_ref::<u64>().cloned().unwrap();
                    Reverse(timestamp)
                });
                Ok(Some(items))
            },
            None,
            Some(Duration::from_secs(1)),
            update_receiver,
            true,
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
    while let Some(Ok(response)) = sync_stream.next().compat().await {
        if response.rooms.is_empty() {
            continue;
        }

        let list: Vec<Item> = ss
            .get_all_rooms()
            .compat()
            .await
            .iter()
            .filter_map(|room| {
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
                    Some(source) => {
                        Img::list_url(mxc_to_http(server.clone(), OwnedMxcUri::from(source), true))
                    }
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

                Some(Item::new(
                    room_id,
                    vec![name.clone()],
                    cx.new_view(|_| ListItem::new(Some(img), name.clone(), None, vec![]))
                        .unwrap()
                        .into(),
                    Some((
                        0.66,
                        Box::new(move |cx| StateItem::init(preview.clone(), false, cx)),
                    )),
                    vec![],
                    None,
                    Some(Box::new(timestamp)),
                    None,
                ))
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

    Ok(())
}

impl RootCommandBuilder for MatrixCommandBuilder {
    fn build(&self, cx: &mut WindowContext) -> RootCommand {
        let view = cx.new_view(|cx| {
            let db = db();
            let sessions = Session::all(db).query().unwrap_or_default();
            for session in sessions {
                cx.spawn(move |view, cx| sync(session.contents, view, cx))
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
