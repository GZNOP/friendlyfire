use std::collections::{HashMap, HashSet};

use friendlyfire_shared_lib::ServerMessage;
use tokio::sync::{Mutex, RwLock, mpsc::UnboundedSender};
use uuid::Uuid;

use crate::{http::storage::FsOverlayStorage, party::PartyId, store::MockStore, user::UserId};

type WsTx = UnboundedSender<ServerMessage>;
type PartySockets = HashMap<PartyId, HashMap<UserId, WsTx>>;

pub type OverlayJobId = Uuid;

pub struct Jobs {
    inner: Mutex<HashMap<OverlayJobId, Job>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JobKind {
    Overlays,
    Rasterization,
}

pub struct Job {
    pub kind: JobKind,
    pub party_id: PartyId,
    pub sender: UserId,
    pub pending: HashSet<UserId>,
}

pub struct JobCompleted {
    pub kind: JobKind,
    pub party_id: PartyId,
    pub sender: UserId,
    pub job_id: Uuid,
}

impl Jobs {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub async fn new_overlay_job(
        &self,
        party_id: PartyId,
        sender: UserId,
        participants: HashSet<UserId>,
    ) {
        let job_id = Uuid::new_v4();
        let job = Job {
            kind: JobKind::Overlays,
            party_id,
            sender,
            pending: participants,
        };

        self.inner.lock().await.insert(job_id, job);
    }

    pub async fn new_rasterization_job(
        &self,
        party_id: PartyId,
        sender: UserId,
        participants: HashSet<UserId>,
    ) {
        let job_id = Uuid::new_v4();
        let job = Job {
            kind: JobKind::Rasterization,
            party_id,
            sender,
            pending: participants,
        };

        self.inner.lock().await.insert(job_id, job);
    }

    /// ACK from a user.
    /// Returns Some(sender_id) if the job is completed by this ACK.
    pub async fn ack(&self, job_id: OverlayJobId, user_id: UserId) -> Option<JobCompleted> {
        let mut jobs = self.inner.lock().await;

        let job = jobs.get_mut(&job_id)?;

        job.pending.remove(&user_id);

        if job.pending.is_empty() {
            let completed = JobCompleted {
                kind: job.kind,
                job_id,
                party_id: job.party_id,
                sender: job.sender,
            };

            jobs.remove(&job_id);
            Some(completed)
        } else {
            None
        }
    }
}

pub struct WsRegistry {
    inner: RwLock<PartySockets>,
}

impl WsRegistry {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add(&self, party_id: PartyId, user_id: UserId, tx_channel: WsTx) {
        let mut inner = self.inner.write().await;

        inner
            .entry(party_id)
            .or_insert_with(HashMap::new)
            .insert(user_id, tx_channel);
    }

    pub async fn remove(&self, party_id: PartyId, user_id: UserId) {
        let mut inner = self.inner.write().await;

        if let Some(users) = inner.get_mut(&party_id) {
            users.remove(&user_id);

            // cleanup empty parties
            if users.is_empty() {
                inner.remove(&party_id);
            }
        }
    }

    pub async fn users_in_party(&self, party_id: PartyId) -> Vec<UserId> {
        self.inner
            .read()
            .await
            .get(&party_id)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub async fn send_to(&self, user_id: UserId, msg: ServerMessage) {
        let inner = self.inner.read().await;
        for users in inner.values() {
            if let Some(tx) = users.get(&user_id) {
                let _ = tx.send(msg.clone());
            }
        }
    }

    pub async fn broadcast(&self, party_id: PartyId, msg: ServerMessage) {
        if let Some(users) = self.inner.read().await.get(&party_id) {
            for tx in users.values() {
                let _ = tx.send(msg.clone());
            }
        }
    }
}

pub struct AppState {
    pub store: RwLock<MockStore>,
    pub ws: WsRegistry,
    pub overlays: FsOverlayStorage,
    pub jobs: Jobs,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            store: RwLock::new(MockStore::default()),
            ws: WsRegistry::new(),
            overlays: FsOverlayStorage::new("/tmp"),
            jobs: Jobs::new(),
        }
    }
}
