use crate::party::PartyId;
use anyhow::bail;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncRead;
use tokio::{
    fs::File,
    io::{self},
};
use uuid::Uuid;

pub type OverlayId = Uuid;

/// An overlay on the storage layer
#[derive(Clone)]
pub struct StoredOverlay {
    pub id: OverlayId,
    pub filename: String,
    pub size: u64,
    pub content_type: Option<String>,
}

#[async_trait]
pub trait OverlayStorage: Send + Sync + 'static {
    async fn create_overlay<D>(
        &self,
        party_id: PartyId,
        filename: String,
        content_type: Option<String>,
        data: &mut D,
    ) -> anyhow::Result<StoredOverlay>
    where
        D: AsyncRead + Unpin + Send;

    async fn open_overlay(
        &self,
        overlay_id: OverlayId,
    ) -> anyhow::Result<Box<dyn AsyncRead + Unpin + Send>>;

    async fn delete_overlay(&self, overlay_id: OverlayId) -> anyhow::Result<()>;
}

pub struct FsOverlayStorage {
    root: PathBuf,
}

const ROOT_SUBFOLDER: &str = "parties";
const OVERLAY_EXTENSION: &str = "bin";

impl FsOverlayStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn overlay_path(&self, party_id: PartyId, overlay_id: OverlayId) -> PathBuf {
        self.root
            .join(ROOT_SUBFOLDER)
            .join(party_id.to_string())
            .join(format!("{overlay_id}.{OVERLAY_EXTENSION}"))
    }
}

#[async_trait]
impl OverlayStorage for FsOverlayStorage {
    async fn create_overlay<D>(
        &self,
        party_id: PartyId,
        filename: String,
        content_type: Option<String>,
        data: &mut D,
    ) -> anyhow::Result<StoredOverlay>
    where
        D: AsyncRead + Unpin + Send,
    {
        let overlay_id = Uuid::new_v4();
        let path = self.overlay_path(party_id, overlay_id);

        fs::create_dir_all(path.parent().unwrap()).await?;

        let mut file = File::create(&path).await?;
        let size = io::copy(data, &mut file).await?;

        Ok(StoredOverlay {
            id: overlay_id,
            filename: filename.into(),
            size,
            content_type,
        })
    }

    async fn open_overlay(
        &self,
        overlay_id: OverlayId,
    ) -> anyhow::Result<Box<dyn AsyncRead + Unpin + Send>> {
        let mut dir = fs::read_dir(self.root.join(ROOT_SUBFOLDER)).await?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path().join(format!("{overlay_id}.bin"));

            if path.exists() {
                return Ok(Box::new(File::open(path).await?));
            }
        }

        bail!("Overlay not found")
    }

    async fn delete_overlay(&self, overlay_id: OverlayId) -> anyhow::Result<()> {
        let mut dir = fs::read_dir(self.root.join("parties")).await?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path().join(format!("{overlay_id}.bin"));

            if path.exists() {
                fs::remove_file(path).await?;
                return Ok(());
            }
        }

        // File did not exist, still return Ok
        Ok(())
    }
}
