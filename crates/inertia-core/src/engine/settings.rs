use tracing::{info, warn};

use crate::error::CoreResult;
use crate::storage::{AppSettings, ArchivedFeedItem};

use super::p2p::validate_relay_multiaddr;
use super::{Engine, DEFAULT_P2P_LISTEN_PORT};

impl Engine {
    pub async fn relay_multiaddr(&self) -> Option<String> {
        self.effective_relay().await
    }

    pub async fn get_settings(&self) -> CoreResult<AppSettings> {
        self.store.with(|store| store.get_settings()).await
    }

    pub async fn set_feed_history_enabled(&self, enabled: bool) -> CoreResult<AppSettings> {
        self.store
            .with_mut(|store| store.set_feed_history_enabled(enabled))
            .await?;

        if enabled {
            let ephemeral = self.collect_ephemeral_feed_items().await?;
            for item in ephemeral {
                let archived = ArchivedFeedItem::from(&item);
                self.store
                    .with_mut(|store| store.upsert_feed_archive(&archived))
                    .await?;
            }
        }

        self.get_settings().await
    }

    pub async fn update_settings(
        &self,
        feed_history_enabled: Option<bool>,
        p2p_listen_port: Option<u16>,
        relay_multiaddr: Option<Option<String>>,
        p2p_announce: Option<Option<String>>,
        web_origin: Option<Option<String>>,
    ) -> CoreResult<AppSettings> {
        if let Some(enabled) = feed_history_enabled {
            self.set_feed_history_enabled(enabled).await?;
        }

        if let Some(Some(ref relay)) = relay_multiaddr {
            if !relay.trim().is_empty() {
                validate_relay_multiaddr(relay)?;
            }
        }

        let relay_updated = relay_multiaddr.is_some();

        self.store
            .with_mut(|store| {
                store.update_connection_settings(
                    p2p_listen_port,
                    relay_multiaddr,
                    p2p_announce,
                    web_origin,
                )?;
                store.get_settings()
            })
            .await?;

        if relay_updated && self.p2p.lock().await.is_some() {
            if let Err(e) = self.redial_known_peers().await {
                warn!(error = %e, "redial after relay settings change failed");
            }
        }

        self.get_settings().await
    }

    pub(super) async fn resolve_listen_port(&self) -> u16 {
        if let Some(port) = super::p2p_listen_port_from_env() {
            return port;
        }
        self.store
            .with(|s| s.get_settings())
            .await
            .map(|s| s.p2p_listen_port)
            .unwrap_or(DEFAULT_P2P_LISTEN_PORT)
    }

    pub(super) async fn effective_relay(&self) -> Option<String> {
        if let Ok(raw) = std::env::var("INERTIA_RELAY") {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        self.store
            .with(|s| s.get_settings())
            .await
            .ok()
            .and_then(|s| s.relay_multiaddr)
    }

    /// Apply relay from a signed invite when env override is not set.
    pub(super) async fn apply_relay_from_invite(&self, relay: &str) -> CoreResult<()> {
        if std::env::var("INERTIA_RELAY")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .is_some()
        {
            return Ok(());
        }
        validate_relay_multiaddr(relay)?;
        self.update_settings(
            None,
            None,
            Some(Some(relay.to_string())),
            None,
            None,
        )
        .await?;
        info!("applied relay multiaddr from invite");
        Ok(())
    }

    pub(super) async fn resolve_invite_web_origin(&self, request_origin: Option<&str>) -> Option<String> {
        if let Ok(raw) = std::env::var("INERTIA_WEB_ORIGIN") {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        if let Ok(settings) = self.store.with(|s| s.get_settings()).await {
            if let Some(origin) = settings.web_origin {
                let trimmed = origin.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        request_origin
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }
}
