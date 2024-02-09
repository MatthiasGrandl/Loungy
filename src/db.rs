use bonsaidb::{
    core::{
        document::KeyId,
        keyvalue::{KeyStatus, KeyValue},
    },
    local::{
        config::{Builder, StorageConfiguration},
        vault::LocalVaultKeyStorage,
        Database,
    },
};
use gpui::{AppContext, Global};
use serde::{de, Serialize};

use crate::paths::Paths;

pub struct Db {
    inner: Database,
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
        let inner = Database::open::<()>(config).expect("Failed to open database");

        cx.set_global(Self { inner });
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
