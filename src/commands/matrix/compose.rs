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

use gpui::*;
use matrix_sdk::{
    ruma::{
        events::{room::message::RoomMessageEventContent, MessageLikeEventContent},
        OwnedEventId, OwnedRoomId,
    },
    Client,
};

use crate::{
    components::shared::{Icon, Img, NoView},
    state::{Action, StateViewBuilder, StateViewContext},
};

#[derive(Clone)]
#[allow(dead_code)]
pub(super) enum ComposeKind {
    Message,
    Reply { event_id: OwnedEventId },
    Edit { event_id: OwnedEventId },
    Delete { event_id: OwnedEventId },
}

#[derive(Clone)]
#[allow(dead_code)]
pub(super) struct Compose {
    client: Client,
    room_id: OwnedRoomId,
    kind: ComposeKind,
}

impl Compose {
    pub fn new(client: Client, room_id: OwnedRoomId, kind: ComposeKind) -> Self {
        Self {
            client,
            room_id,
            kind,
        }
    }
}

impl Compose {
    async fn send(&self, content: impl MessageLikeEventContent) -> anyhow::Result<()> {
        let room = self
            .client
            .get_room(&self.room_id)
            .ok_or(anyhow::Error::msg("Room not found"))?;

        room.send(content).await?;
        Ok(())
    }
}

impl StateViewBuilder for Compose {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context.query.set_placeholder("Type a message...", cx);

        let query = context.query.clone();
        let self_clone = self.clone();

        context.actions.update_global(
            vec![Action::new(
                Img::default().icon(Icon::Send),
                "Send Message",
                None,
                move |this, cx| {
                    let query = query.clone();
                    let text = query.get_text(cx);
                    if text.is_empty() {
                        return;
                    }
                    query.set_text("", cx);
                    let mut toast = this.toast.clone();
                    let self_clone = self_clone.clone();
                    cx.spawn(|mut cx| async move {
                        let content = RoomMessageEventContent::text_markdown(text);
                        if self_clone.send(content).await.is_ok() {
                            toast.success("Messagen sent", &mut cx);
                        } else {
                            toast.error("Failed to send message", &mut cx);
                        }
                    })
                    .detach();
                },
                false,
            )],
            cx,
        );

        cx.new_view(|_| NoView).into()
    }
}
