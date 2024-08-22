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

use async_std::{
    stream::StreamExt,
    task::{sleep, spawn, JoinHandle},
};
use futures::{future::Shared, FutureExt};
use jiff::Timestamp;
use std::{rc::Rc, sync::Arc, time::Duration};
use url::Url;

use gpui::*;
use log::debug;
use matrix_sdk::{
    ruma::{
        events::{
            relation::Annotation,
            room::{message::MessageType, MediaSource},
        },
        OwnedUserId,
    },
    Room,
};
use matrix_sdk_ui::{
    sync_service::SyncService,
    timeline::{EventTimelineItem, PaginationOptions, TimelineDetails, TimelineItemContent},
    Timeline,
};

use crate::{
    command,
    components::{
        list::{AsyncListItems, ItemBuilder, ItemComponent, ItemPreset, ListBuilder},
        shared::{Icon, Img, ImgMask},
    },
    date::format_date,
    loader::Loader,
    state::{
        Action, CommandTrait, Shortcut, StateItem, StateModel, StateViewBuilder, StateViewContext,
    },
    theme::Theme,
};

use super::{
    compose::{Compose, ComposeKind},
    mxc::mxc_to_http,
};

#[derive(Clone)]

pub(super) struct ChatRoom {
    pub(super) timeline: Arc<Timeline>,
    pub(super) sync_service: Arc<SyncService>,
    pub(super) room: Arc<Room>,
}

pub trait OnMouseDown: Fn(&MouseDownEvent, &mut WindowContext) + 'static {}
impl<F> OnMouseDown for F where F: Fn(&MouseDownEvent, &mut WindowContext) + 'static {}

#[derive(Clone)]
pub(super) struct Reaction {
    emoji: String,
    count: u16,
    me: bool,
    on_mouse_down: Rc<dyn OnMouseDown>,
}

#[derive(Clone, IntoElement)]
pub(super) struct Reactions(Vec<Reaction>);

impl RenderOnce for Reactions {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        div().flex().children(self.0.into_iter().map(|reaction| {
            div()
                .flex()
                .items_center()
                .py_0p5()
                .px_1()
                .mr_0p5()
                .rounded_lg()
                .border_1()
                .border_color(theme.crust)
                .bg(if reaction.me {
                    theme.surface0
                } else {
                    theme.mantle
                })
                .child(div().child(reaction.emoji.clone()).mr_1())
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text)
                        .font_weight(FontWeight::BOLD)
                        .child(reaction.count.to_string()),
                )
                .on_mouse_down(MouseButton::Left, {
                    let f = reaction.on_mouse_down.clone();
                    move |a, b| (f)(a, b)
                })
        }))
    }
}

#[derive(Clone, IntoElement)]
pub(super) enum MessageContent {
    Text(String),
    Image(ImageSource),
    // Notice(String),
    // Audio(Img),
    // Video(ImageSource),
    // File(String),
    // Emote(String),
}

impl RenderOnce for MessageContent {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        match self {
            MessageContent::Text(t) => t.into_any_element(),
            MessageContent::Image(i) => img(i).w_64().h_48().into_any_element(),
        }
    }
}

#[derive(Clone)]

pub(super) struct Message {
    pub id: String,
    pub sender: String,
    pub avatar: Img,
    pub content: MessageContent,
    pub timestamp: Timestamp,
    pub edited: bool,
    pub me: bool,
    pub reactions: Reactions,
    pub first: bool,
    pub last: bool,
    pub in_reply_to: Option<String>,
    pub meta: AnyModel,
}

impl Message {
    fn actions(&self, timeline: Arc<Timeline>, room: Arc<Room>) -> Vec<Action> {
        let mut actions = vec![Action::new(
            Img::default().icon(Icon::MessageCircleReply),
            "Reply",
            Some(Shortcut::new("r").cmd()),
            {
                let timeline = timeline.clone();
                move |this, cx| {
                    let event = this.get_meta::<EventTimelineItem>(cx).unwrap();
                    let item = StateItem::init(
                        Compose::new(timeline.clone(), ComposeKind::Reply { event }),
                        false,
                        cx,
                    );
                    StateModel::update(|this, cx| this.push_item(item, cx), cx);
                }
            },
            false,
        )];
        if self.me {
            actions.append(&mut vec![
                Action::new(
                    Img::default().icon(Icon::MessageCircleMore),
                    "Edit",
                    Some(Shortcut::new("e").cmd()),
                    {
                        let timeline = timeline.clone();
                        move |this, cx| {
                            let event = this.get_meta::<EventTimelineItem>(cx).unwrap();
                            let item = StateItem::init(
                                Compose::new(timeline.clone(), ComposeKind::Edit { event }),
                                false,
                                cx,
                            );
                            StateModel::update(|this, cx| this.push_item(item, cx), cx);
                        }
                    },
                    false,
                ),
                Action::new(
                    Img::default().icon(Icon::MessageCircleDashed),
                    "Delete",
                    Some(Shortcut::new("backspace").cmd()),
                    {
                        let room = room.clone();
                        move |this, cx| {
                            let event = this.get_meta::<EventTimelineItem>(cx).unwrap();
                            let mut toast = this.toast.clone();
                            let room = room.clone();
                            cx.spawn(|mut cx| async move {
                                match room.redact(event.event_id().unwrap(), None, None).await {
                                    Ok(_) => {
                                        toast.success("Message deleted", &mut cx);
                                    }
                                    Err(err) => {
                                        log::error!("Failed to delete message {:?}", err);
                                        toast.error("failed to delete message", &mut cx);
                                    }
                                }
                            })
                            .detach();
                        }
                    },
                    false,
                ),
            ])
        }
        //
        actions
    }
}

impl ItemComponent for Message {
    fn render(&self, selected: bool, cx: &WindowContext) -> AnyElement {
        let theme = cx.global::<Theme>();
        let show_avatar = !self.me && self.first;
        let show_reactions = !self.reactions.0.is_empty();

        if show_reactions {
            div().mb_8()
        } else {
            div().mb_0p5()
        }
        .flex()
        .child(
            if self.me {
                let mut el = div().ml_auto().rounded_lg();
                if !self.last {
                    el = el.rounded_br_none();
                }
                if !self.first {
                    el = el.rounded_tr_none();
                };
                el
            } else {
                let el = if self.first {
                    div().ml_6().mr_auto().mt_6()
                } else {
                    div().ml_6().mr_auto()
                };
                let mut el = el.rounded_lg();
                if !self.last {
                    el = el.rounded_bl_none();
                };
                if !self.first {
                    el = el.rounded_tl_none();
                };
                el
            }
            .flex_basis(Pixels(0.0))
            .max_w_4_5()
            .p_2()
            .bg(if selected {
                theme.surface0
            } else {
                theme.mantle
            })
            .border_1()
            .border_color(theme.crust)
            .text_sm()
            .relative()
            .child(self.content.clone())
            .child(
                div()
                    .flex()
                    .justify_end()
                    .text_xs()
                    .text_color(theme.subtext0)
                    .child(format_date(self.timestamp, cx)),
            )
            .child(if show_avatar {
                let mut avatar = self.avatar.clone();
                avatar.mask = ImgMask::Circle;
                div()
                    .absolute()
                    .left_neg_6()
                    .top_neg_6()
                    .flex()
                    .items_center()
                    .child(avatar)
                    .child(
                        div()
                            .ml_2()
                            .text_color(theme.lavender)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.sender.clone()),
                    )
            } else {
                div()
            })
            .child(
                div()
                    .absolute()
                    .left_0()
                    .bottom_neg_6()
                    .child(self.reactions.clone()),
            ),
        )
        .into_any_element()
    }
}

fn get_source(
    source: &MediaSource,
    server: Url,
    //mut encrypted: MutexGuard<HashMap<OwnedMxcUri, EncryptedFile>>,
) -> anyhow::Result<Url> {
    match source {
        MediaSource::Encrypted(e) => {
            //encrypted.insert(e.url.clone(), *e.clone());
            //e.url.to_string()
            mxc_to_http(server, e.url.clone(), false)
        }
        MediaSource::Plain(e) => mxc_to_http(server, e.clone(), false),
    }
}

async fn sync(
    timeline: Arc<Timeline>,
    room: Arc<Room>,
    view: WeakView<AsyncListItems>,
    cx: &mut AsyncWindowContext,
) -> anyhow::Result<()> {
    let (mut messages, mut stream) = timeline.subscribe().await;
    let client = room.client();
    let server = client.homeserver();
    let me = client.user_id().unwrap();

    loop {
        let mut prev: Option<OwnedUserId> = None;
        let mut components: Vec<Message> = vec![];

        for m in messages.clone() {
            let Some(m) = m.as_event() else { continue };

            let sender = match m.sender_profile() {
                TimelineDetails::Ready(sender) => sender,
                _ => continue,
            };
            let avatar = match sender
                .avatar_url
                .as_ref()
                .and_then(|source| mxc_to_http(server.clone(), source.clone(), true).ok())
            {
                Some(url) => Img::default().url(url),
                None => Img::default().icon(Icon::User),
            };

            let id = m.event_id().map(|id| id.to_string()).unwrap_or_default();

            let mut message = Message {
                id: id.clone(),
                avatar,
                sender: sender
                    .display_name
                    .clone()
                    .unwrap_or(m.sender().to_string()),
                content: match m.content() {
                    TimelineItemContent::Message(m) => match m.msgtype() {
                        MessageType::Text(t) => MessageContent::Text(t.body.clone()),
                        MessageType::Image(i) => MessageContent::Image(ImageSource::Uri({
                            let Ok(url) = get_source(&i.source, server.clone()) else {
                                continue;
                            };
                            url.to_string().into()
                        })),
                        _ => MessageContent::Text("Unsupported message type".to_string()),
                    },
                    _ => {
                        continue;
                    }
                },
                me: m.is_own(),
                edited: m.latest_edit_json().is_some(),
                reactions: Reactions({
                    m.reactions()
                        .iter()
                        .map(|(emoji, r)| Reaction {
                            emoji: emoji.clone(),
                            count: r.len() as u16,
                            me: r.by_sender(me).count() > 0,
                            on_mouse_down: Rc::new({
                                let annotation = Annotation::new(
                                    m.event_id().unwrap().to_owned(),
                                    emoji.clone(),
                                );
                                let timeline = timeline.clone();
                                move |_, _| {
                                    let timeline = timeline.clone();
                                    let annotation = annotation.clone();
                                    spawn(
                                        async move { timeline.toggle_reaction(&annotation).await },
                                    );
                                }
                            }),
                        })
                        .collect()
                }),
                timestamp: Timestamp::from_second(m.timestamp().as_secs().into()).unwrap(),
                first: false,
                last: false,
                in_reply_to: None,
                meta: cx.new_model(|_| m.clone()).unwrap().into_any(),
            };
            if !prev.as_ref().is_some_and(|s| s.eq(&m.sender())) {
                message.first = true;
                if let Some(p) = components.last_mut() {
                    p.last = true;
                }
                prev = Some(m.sender().to_owned());
            }
            components.push(message);
        }

        if let Some(p) = components.last_mut() {
            p.last = true;
        }

        let items = components
            .into_iter()
            .map(|m| {
                ItemBuilder::new(m.id.clone(), m.clone())
                    .preset(ItemPreset::Plain)
                    .actions(m.actions(timeline.clone(), room.clone()))
                    .meta(m.meta.clone())
                    .build()
            })
            .collect();

        let result = view.update(cx, |this, cx| {
            this.update("messages".to_string(), items, cx);
        });
        if result.is_err() {
            break;
        }

        if let Some(diff) = stream.next().await {
            diff.apply(&mut messages);
        } else {
            break;
        }
    }

    Ok(())
}

command!(ChatRoom);
impl StateViewBuilder for ChatRoom {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context.query.set_placeholder("Search this chat...", cx);

        let view = cx.new_view(|cx| {
            {
                cx.spawn({
                    let timeline = self.timeline.clone();
                    let room = self.room.clone();
                    |view, mut cx| async move {
                        if let Err(err) = sync(timeline, room, view, &mut cx).await {
                            debug!("Updating room failed: {:?}", err);
                        }
                    }
                })
                .detach();
            }
            AsyncListItems::new()
        });

        AsyncListItems::loader(&view, &context.actions, cx);

        let list = ListBuilder::new()
            .reverse()
            .filter({
                |this, cx| {
                    let text = this.query.get_text(cx).to_lowercase();
                    this.items_all
                        .clone()
                        .into_iter()
                        .filter(|item| {
                            let m = item.get_meta::<EventTimelineItem>(cx).unwrap();
                            if let TimelineItemContent::Message(m) = &m.content() {
                                if m.body().to_lowercase().contains(&text) {
                                    return true;
                                }
                            }
                            if let TimelineDetails::Ready(p) = m.sender_profile() {
                                if p.display_name
                                    .clone()
                                    .unwrap_or_default()
                                    .to_lowercase()
                                    .contains(&text)
                                {
                                    return true;
                                }
                            }
                            false
                        })
                        .collect()
                }
            })
            .scroll_handler({
                let timeline = self.timeline.clone();
                let mut future: Option<Shared<JoinHandle<()>>> = None;
                move |ev, _| {
                    if ev.visible_range.start < 5
                        && (future.is_none()
                            || future.as_mut().is_some_and(|f| f.now_or_never().is_some()))
                    {
                        let timeline = timeline.clone();
                        let mut loader = Loader::add();
                        future = Some(
                            spawn(async move {
                                let timeline = timeline.clone();
                                let _ = timeline
                                    .paginate_backwards(PaginationOptions::simple_request(10))
                                    .await;
                                loader.remove();
                                sleep(Duration::from_millis(250)).await;
                            })
                            .shared(),
                        );
                    }
                }
            })
            .build(
                move |_, _, cx| {
                    Ok(Some(
                        view.read(cx).items.values().flatten().cloned().collect(),
                    ))
                },
                context,
                cx,
            );

        list.into()
    }
}
