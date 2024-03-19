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

use std::sync::Arc;

use gpui::*;
use matrix_sdk::ruma::{
    events::{room::message::RoomMessageEventContent, AnyMessageLikeEventContent},
    OwnedEventId,
};
use matrix_sdk_ui::Timeline;

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
    timeline: Arc<Timeline>,
    kind: ComposeKind,
}

impl Compose {
    pub fn new(timeline: Arc<Timeline>, kind: ComposeKind) -> Self {
        Self { timeline, kind }
    }
}

impl Compose {
    async fn send(&self, content: AnyMessageLikeEventContent) -> anyhow::Result<()> {
        self.timeline.send(content).await;
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
                        if self_clone.send(content.into()).await.is_ok() {
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
