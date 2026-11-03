use {
    crate::State,
    futures::{
        SinkExt,
        StreamExt,
    },
    hyper::upgrade::Upgraded,
    hyper_tungstenite::{
        tungstenite::Message as WsMessage,
        WebSocketStream,
    },
    hyper_util::rt::TokioIo,
    loga::ErrContext,
    shared::interface::{
        shared::AccountId,
        wire::s2c::WsEvent,
    },
    std::{
        collections::HashMap,
        sync::{
            atomic::{
                AtomicU64,
                Ordering,
            },
            Arc,
        },
    },
    tokio::sync::{
        mpsc,
        RwLock,
    },
};

pub struct WsBus {
    subscribers: RwLock<HashMap<AccountId, HashMap<u64, mpsc::Sender<WsEvent>>>>,
    next_id: AtomicU64,
}

pub struct Subscription {
    pub account_id: AccountId,
    pub id: u64,
    pub receiver: mpsc::Receiver<WsEvent>,
}

impl WsBus {
    pub fn new() -> Self {
        return Self {
            subscribers: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(0),
        };
    }

    pub async fn subscribe(&self, account_id: AccountId) -> Subscription {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel(256);
        self.subscribers.write().await.entry(account_id).or_insert_with(HashMap::new).insert(id, tx);
        return Subscription {
            account_id: account_id,
            id: id,
            receiver: rx,
        };
    }

    pub async fn unsubscribe(&self, sub: &Subscription) {
        let mut subs = self.subscribers.write().await;
        if let Some(m) = subs.get_mut(&sub.account_id) {
            m.remove(&sub.id);
            if m.is_empty() {
                subs.remove(&sub.account_id);
            }
        }
    }

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

pub async fn run_ws_connection(state: &Arc<State>, account_id: AccountId, ws: WebSocketStream<TokioIo<Upgraded>>) {
    let mut sub = state.ws_bus.subscribe(account_id).await;
    let (mut sink, mut stream) = ws.split();
    let receiver = &mut sub.receiver;
    let write_task = async {
        while let Some(n) = receiver.recv().await {
            let payload = match serde_json::to_string(&n) {
                Ok(p) => p,
                Err(e) => {
                    state
                        .log
                        .log_err(
                            loga::WARN,
                            loga::err(e.to_string()).context("Failed to serialize WS notification"),
                        );
                    continue;
                },
            };
            if sink.send(WsMessage::Text(payload.into())).await.is_err() {
                break;
            }
        }
    };
    let read_task = async {
        while let Some(m) = stream.next().await {
            match m {
                Ok(WsMessage::Close(_)) | Err(_) => break,
                _ => { },
            }
        }
    };
    tokio::select!{
        _ = write_task => {
        },
        _ = read_task => {
        },
    }
    state.ws_bus.unsubscribe(&sub).await;
}
