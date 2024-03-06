use std::{collections::HashMap, future::IntoFuture};

use gpui::*;
use log::{debug, error, info};
use matrix_sdk::{
    room::RoomMember,
    ruma::{
        api::client::sync::sync_events::v4::RoomSubscription,
        events::{
            reaction::ReactionEventContent,
            relation::Annotation,
            room::{
                message::{
                    MessageType, Relation, RoomMessageEventContent,
                    RoomMessageEventContentWithoutRelation,
                },
                redaction::SyncRoomRedactionEvent,
                MediaSource,
            },
            AnySyncMessageLikeEvent, AnySyncTimelineEvent, OriginalSyncMessageLikeEvent,
            SyncMessageLikeEvent,
        },
        OwnedEventId, OwnedMxcUri, OwnedRoomId, OwnedUserId, UInt,
    },
    Client, RoomMemberships, SlidingSync,
};
use time::format_description;
use url::Url;

use crate::{
    components::{
        list::{AsyncListItems, Item, ListBuilder},
        shared::{Icon, Img, ImgMask, NoView},
    },
    state::{Action, Shortcut, StateViewBuilder, StateViewContext},
    theme::Theme,
};

use super::{
    list::{RoomUpdate, RoomUpdateEvent},
    mxc::mxc_to_http,
};

#[derive(Clone)]
pub(super) struct ChatRoom {
    pub(super) updates: Model<RoomUpdate>,
    pub(super) room_id: OwnedRoomId,
}

pub trait OnMouseDown: Fn(&MouseDownEvent, &mut WindowContext) {
    fn clone_box<'a>(&self) -> Box<dyn 'a + OnMouseDown>
    where
        Self: 'a;
}

impl<F> OnMouseDown for F
where
    F: Fn(&MouseDownEvent, &mut WindowContext) + Clone,
{
    fn clone_box<'a>(&self) -> Box<dyn 'a + OnMouseDown>
    where
        Self: 'a,
    {
        Box::new(self.clone())
    }
}

impl<'a> Clone for Box<dyn 'a + OnMouseDown> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

#[derive(Clone)]
pub(super) struct Reaction {
    count: u16,
    me: Option<String>,
    on_mouse_down: Box<dyn OnMouseDown>,
}

#[derive(Clone, IntoElement)]
pub(super) struct Reactions {
    inner: HashMap<String, Reaction>,
}

impl RenderOnce for Reactions {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        div()
            .flex()
            .children(self.inner.into_iter().map(|(emoji, reaction)| {
                div()
                    .flex()
                    .items_center()
                    .py_0p5()
                    .px_1()
                    .mr_0p5()
                    .rounded_lg()
                    .border_1()
                    .border_color(theme.crust)
                    .bg(if reaction.me.is_some() {
                        theme.surface0
                    } else {
                        theme.mantle
                    })
                    .child(div().child(emoji).mr_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text)
                            .font_weight(FontWeight::BOLD)
                            .child(reaction.count.to_string()),
                    )
                    .on_mouse_down(MouseButton::Left, reaction.on_mouse_down.clone())
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

fn human_duration(unix: u64) -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - unix;
    if duration < 60 {
        format!(
            "{} second{} ago",
            duration,
            if duration == 1 { "" } else { "s" }
        )
    } else if duration < 3600 {
        let count = duration / 60;
        format!("{} minute{} ago", count, if count == 1 { "" } else { "s" })
    } else if duration < 86400 {
        let count = duration / 3600;
        format!("{} hour{} ago", count, if count == 1 { "" } else { "s" })
    } else {
        // print date YYYY/MM/DD
        time::OffsetDateTime::from_unix_timestamp(unix as i64)
            .unwrap()
            .format(&format_description::parse("[year]/[month]/[day]").unwrap())
            .unwrap()
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub(super) struct Message {
    pub id: String,
    pub room_id: String,
    pub sender: String,
    pub avatar: Img,
    pub content: MessageContent,
    pub timestamp: u64,
    pub edited: bool,
    pub me: bool,
    pub reactions: Reactions,
    pub first: bool,
    pub last: bool,
    pub in_reply_to: Option<String>,
}

impl Message {
    fn render(&mut self, selected: bool, cx: &WindowContext) -> Div {
        let theme = cx.global::<Theme>();
        let show_avatar = !self.me && self.first;
        let show_reactions = !self.reactions.inner.is_empty();

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
                    .child(human_duration(self.timestamp)),
            )
            .child(if show_avatar {
                let mut avatar = self.avatar.clone();
                avatar.mask = ImgMask::Circle;
                div()
                    .absolute()
                    .z_index(100)
                    .neg_left_6()
                    .neg_top_6()
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
                    .neg_bottom_6()
                    .child(self.reactions.clone()),
            ),
        )
    }
    fn actions(&self, _client: &Client, _cx: &mut AsyncWindowContext) -> Vec<Action> {
        let mut actions = vec![Action::new(
            Img::list_icon(Icon::MessageCircleReply, None),
            "Reply",
            Some(Shortcut::cmd("r")),
            move |_, _cx| {
                info!("Reply to message");
            },
            false,
        )];
        if self.me {
            actions.append(&mut vec![
                Action::new(
                    Img::list_icon(Icon::MessageCircleMore, None),
                    "Edit",
                    Some(Shortcut::cmd("e")),
                    move |_, _cx| {
                        info!("Edit message");
                    },
                    false,
                ),
                Action::new(
                    Img::list_icon(Icon::MessageCircleDashed, None),
                    "Delete",
                    Some(Shortcut::cmd("backspace")),
                    move |_, _cx| {
                        info!("Delete message");
                    },
                    false,
                ),
            ])
        }
        //
        actions
    }
}

fn get_source(
    source: MediaSource,
    server: Url,
    //mut encrypted: MutexGuard<HashMap<OwnedMxcUri, EncryptedFile>>,
) -> String {
    match source {
        MediaSource::Encrypted(e) => {
            //encrypted.insert(e.url.clone(), *e.clone());
            //e.url.to_string()
            mxc_to_http(server, e.url, false).to_string()
        }
        MediaSource::Plain(e) => mxc_to_http(server, e, false).to_string(),
    }
}

fn get_content(
    content: RoomMessageEventContentWithoutRelation,
    server: Url,
    //encrypted: MutexGuard<HashMap<OwnedMxcUri, EncryptedFile>>,
) -> MessageContent {
    match content.msgtype {
        MessageType::Text(t) => MessageContent::Text(t.body),
        MessageType::Image(i) => {
            MessageContent::Image(ImageSource::Uri(get_source(i.source, server).into()))
        }
        e => MessageContent::Text(format!("msgtype not yet supported: {}", e.msgtype())),
    }
}

async fn sync(
    room_id: OwnedRoomId,
    model: Model<RoomUpdate>,
    view: WeakView<AsyncListItems>,
    cx: &mut AsyncWindowContext,
) -> anyhow::Result<()> {
    let (client, sliding_sync) = cx
        .read_model::<RoomUpdate, (Client, SlidingSync)>(&model, |this, _| {
            (this.client.clone(), this.sliding_sync.clone())
        })?;

    let room = sliding_sync
        .get_room(&room_id)
        .await
        .ok_or(anyhow::Error::msg("Room not found"))?;

    let server = client.homeserver();

    let r = client
        .get_room(&room_id)
        .ok_or(anyhow::Error::msg("Room not found"))?;

    let members = r.members_no_sync(RoomMemberships::all()).await?;

    let me = client.user_id().unwrap();
    let mut member_map: HashMap<OwnedUserId, RoomMember> = HashMap::new();
    for member in members {
        member_map.insert(member.user_id().to_owned(), member);
    }

    let mut prev: Option<OriginalSyncMessageLikeEvent<RoomMessageEventContent>> = None;
    let mut messages: HashMap<OwnedEventId, Message> = HashMap::new();
    let timeline = room.timeline_queue().into_iter();
    let mut reactions: HashMap<OwnedEventId, (OwnedEventId, String, bool)> = HashMap::new();

    for ev in timeline {
        let m = ev.event.deserialize_as::<AnySyncTimelineEvent>();
        if m.is_err() {
            continue;
        }
        let m = m.unwrap();
        if let AnySyncTimelineEvent::MessageLike(e) = m {
            match e {
                AnySyncMessageLikeEvent::Reaction(SyncMessageLikeEvent::Original(e)) => {
                    let emoji = e.content.relates_to.key;
                    let id = e.content.relates_to.event_id;
                    let me = e.sender.to_string().eq(&me.to_string());
                    reactions.insert(e.event_id, (id, emoji, me));
                }
                AnySyncMessageLikeEvent::RoomRedaction(SyncRoomRedactionEvent::Original(e)) => {
                    if let Some(id) = e.content.redacts {
                        _ = reactions.remove(&id);
                        _ = messages.remove(&id);
                    }
                }
                AnySyncMessageLikeEvent::RoomMessage(SyncMessageLikeEvent::Original(e)) => {
                    let in_reply_to = match e.content.clone().relates_to {
                        Some(Relation::Replacement(r)) => {
                            if let Some(m) = messages.get_mut(&r.event_id) {
                                m.edited = true;
                                m.content = get_content(r.new_content, server.clone());
                            };
                            continue;
                        }
                        Some(Relation::Reply { in_reply_to }) => {
                            Some(in_reply_to.event_id.to_string())
                        }
                        _ => None,
                    };
                    let clone = e.clone();
                    let sender = e.sender.clone();
                    let id = e.event_id;
                    let me = me.to_string().eq(&sender);
                    let (sender, avatar) = member_map
                        .clone()
                        .get(&sender)
                        .map(|m| {
                            (
                                m.display_name().unwrap_or(sender.as_ref()).to_string(),
                                match m.avatar_url() {
                                    Some(a) => Img::list_url(mxc_to_http(
                                        server.clone(),
                                        OwnedMxcUri::from(a),
                                        true,
                                    )),
                                    None => Img::list_icon(Icon::User, None),
                                },
                            )
                        })
                        .unwrap_or((sender.to_string(), Img::list_icon(Icon::User, None)));
                    let content = get_content(e.content.into(), server.clone());
                    let room_id = room.room_id().to_string();

                    let mut first = true;
                    if let Some(p) = prev {
                        if p.sender != e.sender {
                            if let Some(m) = messages.get_mut(&p.event_id) {
                                m.last = true;
                            }
                        } else {
                            first = false;
                        }
                    }
                    prev = Some(clone);

                    messages.insert(
                        id.clone(),
                        Message {
                            id: id.to_string(),
                            room_id: room_id.clone(),
                            timestamp: e.origin_server_ts.as_secs().into(),
                            content,
                            sender,
                            avatar,
                            me,
                            edited: false,
                            reactions: Reactions {
                                inner: HashMap::new(),
                            },
                            first,
                            last: false,
                            in_reply_to,
                        },
                    );
                }
                _ => {}
            }
        }
    }
    if let Some(p) = prev {
        if let Some(m) = messages.get_mut(&p.event_id) {
            m.last = true;
        }
    }

    reactions.into_iter().for_each(|(eid, (id, emoji, me))| {
        if let Some(m) = messages.get_mut(&id) {
            let on_mouse_down: Box<dyn OnMouseDown> = {
                let room = r.clone();
                let id = id.clone();
                let reaction_id = eid.clone();
                let emoji = emoji.clone();
                if me {
                    Box::new({
                        move |_, cx| {
                            let reaction_id = reaction_id.clone();
                            let room = room.clone();
                            cx.spawn(move |_| async move {
                                let _ = room.redact(&reaction_id, None, None).await;
                            })
                            .detach();
                        }
                    })
                } else {
                    Box::new({
                        move |_, cx| {
                            let room = room.clone();
                            let id = id.clone();
                            let emoji = emoji.clone();
                            cx.spawn(move |_| async move {
                                let content = ReactionEventContent::new(Annotation::new(
                                    id.clone(),
                                    emoji.clone(),
                                ));
                                let _ = room.send(content).into_future().await;
                            })
                            .detach();
                        }
                    })
                }
            };
            match m.reactions.inner.get_mut(&emoji) {
                Some(r) => {
                    r.count += 1;
                    if me {
                        r.me = Some(eid.to_string());
                        r.on_mouse_down = on_mouse_down;
                    }
                }
                None => {
                    let mut reaction = Reaction {
                        count: 1,
                        me: None,
                        on_mouse_down,
                    };
                    if me {
                        reaction.me = Some(eid.to_string());
                    };
                    m.reactions.inner.insert(emoji.clone(), reaction);
                }
            }
        }
    });

    let mut messages: Vec<Message> = messages.into_values().collect();
    messages.sort_unstable_by_key(|m| m.timestamp);

    let items: Vec<Item> = messages
        .into_iter()
        .map(|m| {
            Item::new(
                m.id.clone(),
                vec![m.sender.clone()],
                cx.new_view(|_| NoView).unwrap().into(),
                None,
                m.actions(&client, cx),
                None,
                Some(Box::new(m)),
                Some(|this, selected, cx| {
                    let message: &Message = this.meta.value().downcast_ref().unwrap();
                    message.clone().render(selected, cx)
                }),
            )
        })
        .collect();

    view.update(cx, |this, cx| {
        this.update("messages".to_string(), items, cx);
    })?;

    Ok(())
}

impl StateViewBuilder for ChatRoom {
    fn build(&self, context: &mut StateViewContext, cx: &mut WindowContext) -> AnyView {
        context.query.set_placeholder("Search this chat...", cx);

        let sliding_sync = self.updates.read(cx).sliding_sync.clone();
        let view = cx.new_view(|cx| {
            {
                let id = self.room_id.clone();
                cx.subscribe(&self.updates, move |_, model, event, cx| match event {
                    RoomUpdateEvent::Update(room_id) => {
                        if id.eq(room_id) {
                            let room_id = room_id.clone();
                            let model = model.clone();
                            cx.spawn(move |view, mut cx| async move {
                                if let Err(err) = sync(room_id, model, view, &mut cx).await {
                                    error!("Updating room failed: {:?}", err);
                                }
                            })
                            .detach();
                        }
                    }
                })
                .detach();
            }
            {
                let id = self.room_id.clone();
                debug!("Chat view created");

                let mut subscription = RoomSubscription::default();
                subscription.timeline_limit = Some(UInt::new(300).unwrap());
                {
                    let sliding_sync = sliding_sync.clone();

                    sliding_sync.subscribe_to_room(id.clone(), Some(subscription));
                }
                {
                    let sliding_sync = sliding_sync.clone();
                    let id = id.clone();
                    cx.on_release(move |_, _, _| {
                        sliding_sync.unsubscribe_from_room(id);
                        debug!("Chat view released")
                    })
                    .detach();
                }
            }
            AsyncListItems::new()
        });

        AsyncListItems::loader(&view, &context.actions, cx);

        let list = ListBuilder::new().reverse().build(
            move |_, _, cx| {
                Ok(Some(
                    view.read(cx).items.values().flatten().cloned().collect(),
                ))
            },
            Some(Box::new(move |this, cx| {
                this.items_all
                    .clone()
                    .into_iter()
                    .filter(|item| {
                        let text = this.query.get_text(cx).to_lowercase();
                        let message: &Message = item.meta.value().downcast_ref().unwrap();
                        if let MessageContent::Text(t) = &message.content {
                            if t.to_lowercase().contains(&text) {
                                return true;
                            }
                        }
                        message.sender.to_lowercase().contains(&text)
                    })
                    .collect()
                //
            })),
            context,
            cx,
        );

        list.into()
    }
}
