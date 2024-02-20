use bonsaidb::{
    core::schema::{Collection, SerializedCollection},
    local::Database,
};
use gpui::*;
use matrix_sdk::{
    matrix_auth::MatrixSession,
    ruma::{
        api::client::sync::sync_events::v4::SyncRequestListFilters,
        events::{StateEventType, TimelineEventType},
    },
    Client, SlidingSync, SlidingSyncList, SlidingSyncMode,
};
use serde::{Deserialize, Serialize};

use crate::{db::Db, paths::paths};

#[derive(Serialize, Deserialize, Collection)]
#[collection(name = "matrix.sessions")]
pub(super) struct Session {
    #[natural_id]
    id: String,
    inner: MatrixSession,
    passphrase: String,
}

impl Session {
    pub(super) async fn load(&self) -> anyhow::Result<(Client, SlidingSync)> {
        let db = paths()
            .data
            .join("matrix")
            .join(self.inner.meta.user_id.to_string());

        let builder = matrix_sdk::Client::builder()
            .server_name(self.inner.meta.user_id.server_name())
            .sqlite_store(db, Some(&self.passphrase.clone()));

        let client = builder.build().await?;
        client
            .matrix_auth()
            .restore_session(self.inner.clone())
            .await?;

        if !client.logged_in() {
            todo!("Prompt for login");
        }

        let mut filter = SyncRequestListFilters::default();
        filter.not_room_types = vec![String::from("m.space")];

        let list = SlidingSyncList::builder("list")
            .sync_mode(SlidingSyncMode::Growing {
                batch_size: (20),
                maximum_number_of_rooms_to_fetch: Some(200),
            })
            .bump_event_types(&[TimelineEventType::RoomMessage])
            .filters(Some(filter))
            .timeline_limit(10)
            .sort(vec![String::from("by_recency")])
            .required_state(vec![
                (StateEventType::RoomAvatar, String::from("")),
                (StateEventType::RoomTopic, String::from("")),
            ]);

        let sliding_sync = client
            .sliding_sync("sync")
            .unwrap()
            .add_cached_list(list)
            .await?
            .with_all_extensions()
            .build()
            .await?;

        Ok((client, sliding_sync))
    }
    pub(super) fn init(cx: &mut WindowContext) -> anyhow::Result<()> {
        // Db::new::<Self, SessionDb>(|db| SessionDb { inner: db }, cx);
        // let all = Self::all(&cx.global::<SessionDb>().inner).query()?;
        // for session in all {
        //     let _ = session.contents.load();
        // }
        Ok(())
    }
}

pub(super) struct SessionDb {
    pub(super) inner: Database,
}
impl Global for SessionDb {}
