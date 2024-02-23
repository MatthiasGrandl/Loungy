use std::{cmp::Reverse, collections::HashMap};

use async_compat::CompatExt;
use gpui::*;
use log::{debug, error, info};
use matrix_sdk::{
    room::RoomMember,
    ruma::{
        api::client::sync::sync_events::v4::RoomSubscription,
        events::{
            room::{
                message::{
                    MessageType, Relation, RoomMessageEventContent,
                    RoomMessageEventContentWithoutRelation,
                },
                redaction::SyncRoomRedactionEvent,
                MediaSource,
            },
            AnySyncMessageLikeEvent, AnySyncTimelineEvent, OriginalSyncMessageLikeEvent,
        },
        OwnedEventId, OwnedMxcUri, OwnedRoomId, OwnedUserId, UInt,
    },
    Client, RoomMemberships, SlidingSync,
};
use time::format_description;
use url::Url;

use crate::{
    components::{
        list::{AsyncListItems, Item, List},
        shared::{Icon, Img, ImgMask},
    },
    query::TextInputWeak,
    state::{ActionsModel, StateViewBuilder},
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

#[derive(Clone)]
pub(super) struct Reaction {
    pub count: u16,
    pub me: Option<String>,
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
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
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
pub struct Message {
    pub id: String,
    pub room_id: String,
    pub sender: String,
    pub avatar: Img,
    pub content: MessageContent,
    pub timestamp: u64,
    pub edited: bool,
    pub me: bool,
    pub reactions: HashMap<String, Reaction>,
    pub first: bool,
    pub last: bool,
    pub in_reply_to: Option<String>,
}

impl Message {
    fn render(&mut self, selected: bool, cx: &WindowContext) -> Div {
        let theme = cx.global::<Theme>();
        let show_avatar = !self.me && self.first;

        div().flex().child(
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
                    div().ml_4().mr_auto().mt_4()
                } else {
                    div().ml_4().mr_auto()
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
            .mb_0p5()
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
                    .neg_left_4()
                    .neg_top_4()
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
            }),
        )
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

    let members = client
        .get_room(&room_id)
        .ok_or(anyhow::Error::msg("Room not found"))?
        .members_no_sync(RoomMemberships::all())
        .compat()
        .await?;

    let me = client.user_id().unwrap();
    let mut member_map: HashMap<OwnedUserId, RoomMember> = HashMap::new();
    for member in members {
        member_map.insert(member.user_id().to_owned(), member);
    }

    let mut prev: Option<OriginalSyncMessageLikeEvent<RoomMessageEventContent>> = None;
    let mut messages: HashMap<String, Message> = HashMap::new();
    let mut timeline = room.timeline_queue().into_iter();
    let mut reactions: HashMap<OwnedEventId, (String, String, bool)> = HashMap::new();

    while let Some(ev) = timeline.next() {
        let m = ev.event.deserialize_as::<AnySyncTimelineEvent>();
        if m.is_err() {
            continue;
        }
        let m = m.unwrap();
        match m {
            AnySyncTimelineEvent::MessageLike(e) => match e {
                AnySyncMessageLikeEvent::Reaction(e) => match e {
                    matrix_sdk::ruma::events::SyncMessageLikeEvent::Original(e) => {
                        let emoji = e.content.relates_to.key;
                        let id = e.content.relates_to.event_id.to_string();
                        let me = e.sender.to_string().eq(&me.to_string());
                        reactions.insert(e.event_id, (id, emoji, me));
                    }
                    _ => {}
                },
                AnySyncMessageLikeEvent::RoomRedaction(e) => match e {
                    SyncRoomRedactionEvent::Original(e) => {
                        if let Some(id) = e.content.redacts {
                            _ = reactions.remove(&id);
                            _ = messages.remove(&id.to_string());
                        }
                    }
                    _ => {}
                },
                AnySyncMessageLikeEvent::RoomMessage(e) => match e {
                    matrix_sdk::ruma::events::SyncMessageLikeEvent::Original(e) => {
                        let in_reply_to = match e.content.clone().relates_to {
                            Some(Relation::Replacement(r)) => {
                                if let Some(m) = messages.get_mut(&r.event_id.to_string()) {
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
                        let id = e.event_id.to_string();
                        let me = me.to_string().eq(&sender);
                        let (sender, avatar) = member_map
                            .clone()
                            .get(&sender)
                            .map(|m| {
                                (
                                    m.display_name().unwrap_or(&sender.to_string()).to_string(),
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
                                if let Some(m) = messages.get_mut(&p.event_id.to_string()) {
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
                                id,
                                room_id,
                                timestamp: e.origin_server_ts.as_secs().into(),
                                content,
                                sender,
                                avatar,
                                me,
                                edited: false,
                                reactions: HashMap::new(),
                                first,
                                last: false,
                                in_reply_to,
                            },
                        );
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
    if let Some(p) = prev {
        if let Some(m) = messages.get_mut(&p.event_id.to_string()) {
            m.last = true;
        }
    }

    reactions.into_iter().for_each(|(eid, (id, emoji, me))| {
        if let Some(m) = messages.get_mut(&id) {
            match m.reactions.get_mut(&emoji) {
                Some(r) => {
                    r.count += 1;
                    if me {
                        r.me = Some(eid.to_string());
                    }
                }
                None => {
                    let mut reaction = Reaction { count: 1, me: None };
                    if me {
                        reaction.me = Some(eid.to_string());
                    };
                    m.reactions.insert(emoji.clone(), reaction);
                }
            }
        }
    });

    let mut messages: Vec<Message> = messages.into_iter().map(|(_, v)| v).collect();
    messages.sort_unstable_by_key(|m| m.timestamp);

    let items: Vec<Item> = messages
        .into_iter()
        .map(|m| {
            info!("{:?}", m.sender);
            Item::new(
                vec![m.sender.clone()],
                cx.new_view(|_| NoView).unwrap().into(),
                None,
                vec![],
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

struct NoView;
impl Render for NoView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
    }
}

impl StateViewBuilder for ChatRoom {
    fn build(
        &self,
        query: &TextInputWeak,
        actions: &ActionsModel,
        update_receiver: std::sync::mpsc::Receiver<bool>,
        cx: &mut WindowContext,
    ) -> AnyView {
        query.set_placeholder("Search your rooms...", cx);

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

        AsyncListItems::loader(&view, &actions, cx);

        let list = List::new(
            query,
            &actions,
            move |_, _, cx| {
                Ok(Some(
                    view.read(cx).items.values().flatten().cloned().collect(),
                ))
            },
            None,
            None,
            update_receiver,
            true,
            cx,
        );
        list.update(cx, |this, cx| {
            this.change_alignment(ListAlignment::Bottom, cx);
        });

        list.into()
    }
}
