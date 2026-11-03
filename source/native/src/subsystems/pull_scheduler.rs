use {
    crate::{
        db,
        dbutil::tx,
        subsystems::messages::{
            posting_members,
            PullOutcome,
        },
        State,
    },
    loga::ResultContext,
    rand::RngExt,
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
        sync::Arc,
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
pub const PULL_MAX_BACKOFF: Duration = Duration::from_secs(3600);
const PULL_JITTER_FRAC: f64 = 0.25;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct PullKey {
    pub account_id: AccountId,
    pub owner: Identity,
    pub channel: ChannelId,
    pub source: Identity,
}

struct Entry {
    at: Instant,
    key: PullKey,
    version: u64,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        return self.at == other.at;
    }
}

impl Eq for Entry {}

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

struct KeyState {
    backoff: Duration,
    version: u64,
    running: bool,
    rerun: bool,
}

struct Inner {
    queue: BinaryHeap<Reverse<Entry>>,
    state: HashMap<PullKey, KeyState>,
}

impl Inner {
    fn schedule(&mut self, key: PullKey, at: Instant, backoff: Duration) {
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
    inner: Mutex<Inner>,
    wake: Notify,
}

impl PullScheduler {
    pub fn new() -> Arc<Self> {
        return Arc::new(Self {
            inner: Mutex::new(Inner {
                queue: BinaryHeap::new(),
                state: HashMap::new(),
            }),
            wake: Notify::new(),
        });
    }

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

    pub async fn remove_channel(&self, account_id: AccountId, owner: Identity, channel: &ChannelId) {
        self
            .inner
            .lock()
            .await
            .state
            .retain(|k, _| !(k.account_id == account_id && k.owner == owner && &k.channel == channel));
    }

    async fn finish(&self, key: PullKey, outcome: PullOutcome) {
        {
            let mut inner = self.inner.lock().await;
            let Some(s) = inner.state.get_mut(&key) else {
                return;
            };
            s.running = false;
            let backoff = s.backoff;
            let rerun = s.rerun;
            s.rerun = false;
            let next_backoff = match outcome {
                PullOutcome::Gone => {
                    inner.state.remove(&key);
                    return;
                },
                PullOutcome::New => PULL_MIN_BACKOFF,
                PullOutcome::NoNew => backoff.saturating_mul(2).min(PULL_MAX_BACKOFF),
            };
            let at = if rerun {
                Instant::now()
            } else {
                Instant::now() + jitter(next_backoff)
            };
            inner.schedule(key, at, next_backoff);
        }
        self.wake.notify_one();
    }

    pub async fn run(self: Arc<Self>, state: Arc<State>) -> Result<(), loga::Error> {
        loop {
            let next_at = {
                let mut inner = self.inner.lock().await;
                let mut next_at = None;
                loop {
                    let Some(Reverse(top)) = inner.queue.peek() else {
                        break;
                    };
                    let live = match inner.state.get(&top.key) {
                        Some(s) => s.version == top.version && !s.running,
                        None => false,
                    };
                    if !live {
                        inner.queue.pop();
                        continue;
                    }
                    if top.at > Instant::now() {
                        next_at = Some(top.at);
                        break;
                    }
                    let Some(Reverse(entry)) = inner.queue.pop() else {
                        break;
                    };
                    inner.state.get_mut(&entry.key).unwrap().running = true;
                    let scheduler = self.clone();
                    let state = state.clone();
                    tokio::spawn(async move {
                        let outcome = execute_pull(&state, &entry.key).await;
                        scheduler.finish(entry.key, outcome).await;
                    });
                }
                next_at
            };
            match next_at {
                None => {
                    self.wake.notified().await;
                },
                Some(at) => {
                    tokio::select!{
                        _ = tokio::time::sleep_until(at) => {},
                        _ = self.wake.notified() => {},
                    };
                },
            }
        }
    }
}

fn jitter(d: Duration) -> Duration {
    let frac: f64 = rand::rng().random_range(-PULL_JITTER_FRAC .. PULL_JITTER_FRAC);
    let secs = d.as_secs_f64() * (1.0 + frac);
    return Duration::from_secs_f64(secs.max(1.0));
}

async fn execute_pull(state: &Arc<State>, key: &PullKey) -> PullOutcome {
    match crate::subsystems::messages::pull_and_store(state, key).await {
        Ok(outcome) => return outcome,
        Err(e) => {
            state.log.log_err(loga::WARN, e.context("Error executing scheduled pull"));
            return PullOutcome::NoNew;
        },
    }
}

pub async fn enumerate_and_seed(state: &Arc<State>) -> Result<(), loga::Error> {
    let keys = tx(&state.db, move |db_tx| {
        let channels =
            good_ormning::sqlite::good_query_many!(
                db,
                r#"select account_id, identity, id, local_identity from channel where deleted is null"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        let mut keys = vec![];
        for c in channels {
            if c.local_identity.0 == c.identity.0 {
                for member in posting_members(db_tx, c.account_id, c.identity, &c.id)? {
                    keys.push(PullKey {
                        account_id: c.account_id.0,
                        owner: c.identity.0,
                        channel: c.id.0.clone(),
                        source: member,
                    });
                }
            } else {
                keys.push(PullKey {
                    account_id: c.account_id.0,
                    owner: c.identity.0,
                    channel: c.id.0.clone(),
                    source: c.identity.0,
                });
            }
        }
        return Ok(keys);
    }).await.context("Error enumerating channels for scheduler seed")?;
    for key in keys {
        state.pull_scheduler.add_or_reset(key).await;
    }
    return Ok(());
}
