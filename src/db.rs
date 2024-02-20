use std::sync::OnceLock;

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
use serde::{de, Serialize};

use crate::paths::paths;

#[derive(Clone)]
pub struct Db {
    storage: Storage,
    pub inner: Database,
}

pub fn db() -> &'static Db {
    static DB: OnceLock<Db> = OnceLock::new();
    DB.get_or_init(|| Db::new())
}

impl Db {
    pub fn new() -> Self {
        let data = paths().data.clone();
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

        Self { inner, storage }
    }
    pub fn init_collection<'a, C: Collection + 'static>() -> Database {
        let storage = &db().storage;
        storage
            .register_schema::<C>()
            .expect("Failed to register schema");
        let db = storage
            .create_database::<C>(&C::collection_name().to_string(), true)
            .expect("Failed to open database");
        db
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
