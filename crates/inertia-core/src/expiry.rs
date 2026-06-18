use std::time::Duration;

use tracing::{debug, info};

use crate::error::CoreResult;
use crate::storage::PurgeReport;
use crate::store_handle::StoreHandle;

/// Background task that purges expired outbox/inbox entries.
pub struct ExpiryScheduler {
    store: StoreHandle,
    interval: Duration,
}

impl ExpiryScheduler {
    pub fn new(store: StoreHandle, interval: Duration) -> Self {
        Self { store, interval }
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        let store = self.store;
        let interval = self.interval;
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                match store.with(|s| s.purge_expired()).await {
                    Ok(report)
                        if report.outbox > 0 || report.inbox > 0 || report.invites > 0 =>
                    {
                        info!(
                            outbox = report.outbox,
                            inbox = report.inbox,
                            invites = report.invites,
                            "purged expired content"
                        );
                    }
                    Ok(_) => debug!("expiry sweep: nothing to purge"),
                    Err(e) => tracing::error!(error = %e, "expiry sweep failed"),
                }
            }
        })
    }

    pub async fn run_once(&self) -> CoreResult<PurgeReport> {
        self.store.with(|store| store.purge_expired()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::time::Duration;

    use crate::content::{ContentType, DeliveryStatus};
    use crate::storage::OutboxEntry;

    #[tokio::test]
    async fn purges_expired_outbox() {
        let dir = tempfile::tempdir().unwrap();
        let store = StoreHandle::open(dir.path()).unwrap();

        store
            .with_mut(|s| {
                s.insert_outbox(
                    &OutboxEntry {
                        content_id: "old".into(),
                        recipient_id: "bob".into(),
                        status: DeliveryStatus::Pending,
                        expires_at: Utc::now() - chrono::Duration::hours(1),
                        retry_count: 0,
                        ciphertext: vec![],
                        content_type: ContentType::Message,
                    },
                    "{}",
                )
            })
            .await
            .unwrap();

        let scheduler = ExpiryScheduler::new(store.clone(), Duration::from_secs(60));
        let report = scheduler.run_once().await.unwrap();
        assert_eq!(report.outbox, 1);
        assert!(store.with(|s| s.list_outbox()).await.unwrap().is_empty());
    }
}
