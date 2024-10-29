use crate::config::RoomInfo;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use teloxide::prelude::ChatId;
use tokio::sync::RwLock;

pub static AUTH: Lazy<RwLock<AuthConfig>> = Lazy::new(|| {
    let auth = std::fs::read_to_string("auth.json").unwrap_or("".to_string());
    let users: Vec<User> = serde_json::from_str(&auth).unwrap_or_default();
    RwLock::new(AuthConfig::new(users))
});

#[derive(Serialize, Deserialize)]
pub(crate) struct User {
    /// Chat ID of a telegram user
    pub chat_id: i64,
}

impl User {
    pub(crate) fn new(chat_id: i64) -> Self {
        Self { chat_id }
    }
}

pub struct AuthConfig {
    inner: HashMap<i64, User>,
}

impl AuthConfig {
    fn new(users: Vec<User>) -> Self {
        let inner = users.into_iter().map(|u| (u.chat_id, u)).collect();
        Self { inner }
    }

    pub async fn create(&mut self, id: ChatId) -> anyhow::Result<()> {
        self.inner.entry(id.0).or_insert_with(|| User::new(id.0));
        self.save().await
    }

    pub async fn has_permission(&self, id: ChatId) -> bool {
        self.inner.contains_key(&id.0)
    }

    pub async fn room(&self, id: ChatId) -> RoomInfo {
        let user = self.inner.get(&id.0).expect("User not found");
        RoomInfo::new(user.chat_id)
    }

    async fn save(&self) -> anyhow::Result<()> {
        let auth: Vec<_> = self.inner.values().collect();
        let auth = serde_json::to_string(&auth).expect("Failed to serialize auth");
        Ok(std::fs::write("auth.json", auth)?)
    }
}
