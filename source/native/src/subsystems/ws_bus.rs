use {
    shared::interface::{
        shared::AccountId,
        wire::s2c::WsEvent,
    },
    std::{
        collections::HashMap,
        sync::atomic::AtomicU64,
    },
    tokio::sync::{
        mpsc,
        RwLock,
    },
};

pub struct WsBus {
    pub subscribers: RwLock<HashMap<AccountId, HashMap<u64, mpsc::Sender<WsEvent>>>>,
    pub next_id: AtomicU64,
}

pub struct Subscription {
    pub account_id: AccountId,
    pub id: u64,
    pub receiver: mpsc::Receiver<WsEvent>,
}

impl WsBus {
    pub async fn publish(&self, account_id: AccountId, notification: WsEvent) {
        let subs = self.subscribers.read().await;
        let Some(m) = subs.get(&account_id) else {
            return;
        };
        for tx in m.values() {
            let _ = tx.try_send(notification.clone());
        }
    }
}
