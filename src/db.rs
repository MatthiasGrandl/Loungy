use bonsaidb::{
    core::{
        connection::StorageConnection,
        document::KeyId,
        keyvalue::{KeyStatus, KeyValue},
        schema::Collection,
    },
    local::{
        config::{Builder, StorageConfiguration},
        vault::LocalVaultKeyStorage,
        Database, Storage,
    },
};
use gpui::{AppContext, Global};
use serde::{de, Serialize};

use crate::paths::Paths;

#[derive(Clone)]
pub struct Db {
    storage: Storage,
    pub inner: Database,
}

impl Global for Db {}

impl Db {
    pub fn init(cx: &mut AppContext) {
        let data = cx.global::<Paths>().data.clone();
        let path = data.join("bonsai/db");
        let keys = data.join("bonsai/keys");
        let config = StorageConfiguration::new(path)
            .vault_key_storage(LocalVaultKeyStorage::new(keys).expect("Failed to create vault"))
            .default_encryption_key(KeyId::Master);
        let storage = Storage::open(config).expect("Failed to open storage");
        storage
            .register_schema::<()>()
            .expect("Failed to register schema");
        let inner = storage
            .create_database::<()>("kv", true)
            .expect("Failed to open database");

        cx.set_global(Self { inner, storage });
    }
    pub fn new<'a, C: Collection + 'static, G: Global>(
        f: impl FnOnce(Database) -> G,
        cx: &mut AppContext,
    ) {
        if cx.has_global::<G>() {
            return;
        }
        let storage = cx.global::<Db>().storage.clone();
        storage
            .register_schema::<C>()
            .expect("Failed to register schema");
        let db = storage
            .create_database::<C>(&C::collection_name().to_string(), true)
            .expect("Failed to open database");
        cx.set_global::<G>(f(db));
    }
    pub fn get<T: de::DeserializeOwned>(&self, id: &str) -> Option<T> {
        if let Ok(value) = self.inner.get_key(id).into() {
            value
        } else {
            None
        }
    }
    pub fn set<T: Serialize + Send + Sync>(
        &self,
        id: &str,
        value: &T,
    ) -> anyhow::Result<KeyStatus> {
        Ok(self.inner.set_key(id, value).execute()?)
    }
}
