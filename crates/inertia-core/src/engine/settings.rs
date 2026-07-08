use std::sync::Arc;

use tracing::{info, warn};

use crate::error::CoreResult;
use crate::storage::{AppSettings, ArchivedFeedItem};

use super::relay_list::{merge_relay, relay_lists_equivalent, relays_from_env, validate_relay_list};
use super::{Engine, DEFAULT_P2P_LISTEN_PORT};

impl Engine {
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
        relay_multiaddrs: Option<Vec<String>>,
        p2p_announce: Option<Option<String>>,
        web_origin: Option<Option<String>>,
    ) -> CoreResult<AppSettings> {
        if let Some(enabled) = feed_history_enabled {
            self.set_feed_history_enabled(enabled).await?;
        }

        if let Some(ref relays) = relay_multiaddrs {
            validate_relay_list(relays)?;
        }

        let relay_patch = relay_multiaddrs.is_some();
        let prior_relays = if relay_patch {
            self.effective_relays().await
        } else {
            Vec::new()
        };

        self.store
            .with_mut(|store| {
                store.update_connection_settings(
                    p2p_listen_port,
                    relay_multiaddrs,
                    p2p_announce,
                    web_origin,
                )?;
                store.get_settings()
            })
            .await?;

        if relay_patch {
            if let Err(e) = self.apply_relay_list_to_p2p().await {
                warn!(error = %e, "apply relay list after settings change failed");
            }
            let new_relays = self.effective_relays().await;
            let relay_changed = !relay_lists_equivalent(&prior_relays, &new_relays);
            let needs_reconnect = relay_changed || !self.any_relay_connected().await;
            if needs_reconnect && self.p2p.lock().await.is_some() {
                info!(
                    relay_changed,
                    "background relay bootstrap after settings save"
                );
                let p2p = Arc::clone(&self.p2p);
                let relays = new_relays;
                tokio::spawn(async move {
                    super::relay_dial::bootstrap_relays_for_friend_dial(&p2p, &relays).await;
                });
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

    pub(super) async fn effective_relays(&self) -> Vec<String> {
        let env_relays = relays_from_env();
        if !env_relays.is_empty() {
            return env_relays;
        }
        self.store
            .with(|s| s.get_settings())
            .await
            .map(|s| s.relay_multiaddrs)
            .unwrap_or_default()
    }

    /// Apply relay from a signed invite when env override is not set.
    pub(super) async fn apply_relay_from_invite(&self, relay: &str) -> CoreResult<()> {
        if !relays_from_env().is_empty() {
            return Ok(());
        }
        let current = self.effective_relays().await;
        let merged = merge_relay(&current, relay);
        if merged.len() == current.len() {
            return Ok(());
        }
        validate_relay_list(&merged)?;
        self.store
            .with_mut(|store| {
                store.update_connection_settings(None, Some(merged), None, None)?;
                Ok(())
            })
            .await?;
        self.apply_relay_list_to_p2p().await?;
        info!("merged relay multiaddr from invite");
        Ok(())
    }

    pub(super) async fn apply_relay_list_to_p2p(&self) -> CoreResult<()> {
        let relays = self.effective_relays().await;
        let guard = self.p2p.lock().await;
        if let Some(p2p) = guard.as_ref() {
            p2p.ensure_relay_circuits(&relays).await;
        }
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
