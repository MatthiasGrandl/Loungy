use std::{str::FromStr, sync::OnceLock};

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
        OwnedUserId,
    },
    Client, SlidingSync, SlidingSyncList, SlidingSyncMode,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    db::Db,
    paths::{paths, NAME},
    state::{Actions, StateModel},
};

#[derive(Debug, Serialize, Deserialize, Collection, Clone)]
#[collection(name = "matrix.sessions")]
pub(super) struct Session {
    #[natural_id]
    pub id: String,
    inner: MatrixSession,
    passphrase: String,
}

pub fn db() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(Db::init_collection::<Session>)
}

impl Session {
    pub(super) async fn client(user: &OwnedUserId, password: &str) -> anyhow::Result<Client> {
        let db = paths().data.join("matrix").join(user.to_string());

        let builder = matrix_sdk::Client::builder()
            .server_name(user.server_name())
            .sqlite_store(db, Some(password));

        Ok(builder.build().await?)
    }
    pub(super) async fn load(&self) -> anyhow::Result<(Client, SlidingSync)> {
        let client = Self::client(&self.inner.meta.user_id, &self.passphrase).await?;
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
    pub(super) async fn login(
        username: String,
        password: String,
        mut actions: Actions,
        cx: &mut AsyncWindowContext,
    ) -> anyhow::Result<()> {
        let passphrase: Vec<u8> = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(64)
            .collect();
        let passphrase = String::from_utf8(passphrase)?;
        let user = OwnedUserId::from_str(username.as_str())?;
        let client = Self::client(&user, &passphrase).await?;
        let _ = client
            .matrix_auth()
            .login_username(&username, &password)
            .initial_device_display_name(NAME)
            .send()
            .await?;
        if let Some(session) = client.matrix_auth().session() {
            let session = Session {
                id: username,
                inner: session.clone(),
                passphrase,
            };
            session.push_into(db())?;
            actions.toast.success("Login successfull", cx);
            StateModel::update_async(|this, cx| this.reset(cx), cx);
        } else {
            actions.toast.error("Login failed", cx);
        }
        Ok(())
    }
}
