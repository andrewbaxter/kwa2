use {
    shared::interface::shared::{
        AccountId,
        ChannelId,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cmp::Reverse,
        collections::{
            BinaryHeap,
            HashMap,
        },
        time::Duration,
    },
    tokio::{
        sync::{
            Mutex,
            Notify,
        },
        time::Instant,
    },
};

pub const PULL_MIN_BACKOFF: Duration = Duration::from_secs(5);
pub const PULL_JITTER_FRAC: f64 = 0.25;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct PullKey {
    pub account_id: AccountId,
    pub owner: Identity,
    pub channel: ChannelId,
    pub source: Identity,
}

pub struct Entry {
    pub at: Instant,
    pub key: PullKey,
    pub version: u64,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        return self.at == other.at;
    }
}

impl Eq for Entry { }

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        return Some(self.cmp(other));
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        return self.at.cmp(&other.at);
    }
}

pub struct KeyState {
    pub backoff: Duration,
    pub version: u64,
    pub running: bool,
    pub rerun: bool,
}

pub struct Inner {
    pub queue: BinaryHeap<Reverse<Entry>>,
    pub state: HashMap<PullKey, KeyState>,
}

impl Inner {
    pub fn schedule(&mut self, key: PullKey, at: Instant, backoff: Duration) {
        let entry = self.state.entry(key.clone()).or_insert(KeyState {
            backoff: backoff,
            version: 0,
            running: false,
            rerun: false,
        });
        entry.backoff = backoff;
        entry.version = entry.version.wrapping_add(1);
        let version = entry.version;
        self.queue.push(Reverse(Entry {
            at: at,
            key: key,
            version: version,
        }));
    }
}

pub struct PullScheduler {
    pub inner: Mutex<Inner>,
    pub wake: Notify,
}

impl PullScheduler {
    pub async fn add_or_reset(&self, key: PullKey) {
        self.inner.lock().await.schedule(key, Instant::now() + PULL_MIN_BACKOFF, PULL_MIN_BACKOFF);
        self.wake.notify_one();
    }

    pub async fn trigger_immediate(&self, key: PullKey) {
        {
            let mut inner = self.inner.lock().await;
            match inner.state.get_mut(&key) {
                Some(s) if s.running => {
                    s.rerun = true;
                    return;
                },
                Some(s) => {
                    let backoff = s.backoff;
                    inner.schedule(key, Instant::now(), backoff);
                },
                None => {
                    inner.schedule(key, Instant::now(), PULL_MIN_BACKOFF);
                },
            }
        }
        self.wake.notify_one();
    }

    pub async fn remove(&self, key: &PullKey) {
        self.inner.lock().await.state.remove(key);
    }
}
