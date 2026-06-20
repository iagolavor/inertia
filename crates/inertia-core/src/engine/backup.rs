use crate::error::CoreResult;
use crate::storage::{FeedBackup, FeedRestoreReport};

use super::Engine;

impl Engine {
    pub async fn export_feed_backup(&self) -> CoreResult<FeedBackup> {
        self.store.with(|store| store.export_feed_backup()).await
    }

    pub async fn import_feed_backup(&self, backup: FeedBackup) -> CoreResult<FeedRestoreReport> {
        let report = self
            .store
            .with_mut(|store| store.import_feed_backup(&backup))
            .await?;
        if !self.get_settings().await?.feed_history_enabled {
            self.set_feed_history_enabled(true).await?;
        }
        Ok(report)
    }
}
