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

use std::{cmp::Reverse, collections::HashMap, sync::Arc};

use async_std::{stream::StreamExt, task::spawn};
use bonsaidb::core::schema::SerializedCollection;
use gpui::*;
use matrix_sdk::ruma::OwnedRoomId;
use matrix_sdk_ui::{sync_service::State, timeline::RoomExt};

use crate::{
    command,
    commands::{RootCommand, RootCommandBuilder},
    components::{
        list::{AsyncListItems, Item, ItemBuilder, ListBuilder, ListItem},
        shared::{Icon, Img, ImgMask},
    },
    state::{
        Action, CommandTrait, Shortcut, StateItem, StateModel, StateViewBuilder, StateViewContext,
    },
};

use super::{
    account::AccountCreationBuilder,
    chat::ChatRoom,
    client::{db, Session},
    compose::{Compose, ComposeKind},
    mxc::mxc_to_http,
};

#[derive(Clone)]
struct RoomList {
    view: View<AsyncListItems>,
}

command!(RoomList);
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
                    items.sort_unstable_by_key(|item| Reverse(item.get_meta::<u64>(cx).unwrap()));
                    Ok(Some(items))
                },
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
    let (client, ss) = {
        let session = session.clone();
        spawn(async move { session.load().await }).await?
    };

    ss.start().await;

    let server = client.homeserver();

    let mut previews = HashMap::<OwnedRoomId, ChatRoom>::new();
    let (mut rooms, mut stream) = ss.room_list_service().all_rooms().await?.entries();

    {
        let ss = ss.clone();
        spawn(async move {
            let mut state = ss.state();
            while let Some(state) = state.next().await {
                if state == State::Error {
                    ss.start().await;
                }
            }
        });
    }

    while let Some(diff) = stream.next().await {
        for d in diff {
            d.apply(&mut rooms);
        }
        if rooms.clone().is_empty() {
            continue;
        };

        let mut items: Vec<Item> = vec![];

        for room in rooms.clone() {
            let Some(id) = room.as_room_id() else {
                continue;
            };
            let room = Arc::new(client.get_room(id).unwrap());

            let timeline = Arc::new(room.timeline_builder().build().await);

            let timestamp: u64 = timeline
                .latest_event()
                .await
                .map(|ev| ev.timestamp().as_secs().into())
                .unwrap_or(0);

            let name = room.name().unwrap_or("".to_string());
            let dm = room.is_direct().await?;

            // why is this necessary???
            let avatar = if dm {
                if let Some(m) = room.direct_targets().into_iter().next() {
                    room.get_member_no_sync(&m)
                        .await
                        .ok()
                        .flatten()
                        .and_then(|m| m.avatar_url().map(|m| m.to_owned()))
                } else {
                    None
                }
            } else {
                room.avatar_url()
            };
            let mut img =
                match avatar.and_then(|source| mxc_to_http(server.clone(), source, true).ok()) {
                    Some(source) => Img::default().url(source),
                    None => {
                        if dm {
                            Img::default().icon(Icon::User)
                        } else {
                            Img::default().icon(Icon::Users)
                        }
                    }
                };

            img.mask = ImgMask::Circle;

            let room_id = room.room_id().to_owned();
            let preview = if let Some(preview) = previews.get(&room_id) {
                preview.clone()
            } else {
                let preview = ChatRoom {
                    timeline: timeline.clone(),
                    sync_service: ss.clone(),
                    room: room.clone(),
                };
                previews.insert(room_id.clone(), preview.clone());
                preview
            };

            let item = ItemBuilder::new(
                room_id.clone(),
                ListItem::new(Some(img), name.clone(), None, vec![]),
            )
            .keywords(vec![name.clone()])
            .actions(vec![
                Action::new(
                    Img::default().icon(Icon::MessageCircle),
                    "Write",
                    None,
                    {
                        let timeline = timeline.clone();
                        move |_this, cx| {
                            let item = StateItem::init(
                                Compose::new(timeline.clone(), ComposeKind::Message),
                                false,
                                cx,
                            );
                            StateModel::update(|this, cx| this.push_item(item, cx), cx);
                        }
                    },
                    false,
                ),
                Action::new(
                    Img::default().icon(Icon::Search),
                    "Search",
                    Some(Shortcut::new("/").cmd()),
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
            .meta(cx.new_model(|_| timestamp).unwrap().into_any())
            .build();

            items.push(item);
        }

        let id = session.id.clone();
        if let Some(view) = view.upgrade() {
            view.update(&mut cx, |view, cx| {
                view.update(id, items, cx);
            })?;
        } else {
            break;
        }
    }
    Ok(())
}
command!(MatrixCommandBuilder);
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
            move |_, cx| {
                let view = view.clone();
                let sessions = Session::all(db());
                if sessions.count().unwrap_or_default() == 0 {
                    StateModel::update(|this, cx| this.push(AccountCreationBuilder, cx), cx);
                } else {
                    StateModel::update(|this, cx| this.push(RoomList { view }, cx), cx);
                };
            },
        )
    }
}
