use rusqlite::params;

use crate::error::CoreResult;

use super::sql::{
    FEED_HISTORY_KEY, P2P_ANNOUNCE_KEY, P2P_LISTEN_PORT_KEY, RELAY_MULTIADDR_KEY, WEB_ORIGIN_KEY,
};
use super::{AppSettings, Store};

impl Store {
    pub fn get_settings(&self) -> CoreResult<AppSettings> {
        Ok(AppSettings {
            feed_history_enabled: self.get_bool_setting(FEED_HISTORY_KEY)?.unwrap_or(false),
            p2p_listen_port: self
                .get_string_setting(P2P_LISTEN_PORT_KEY)?
                .and_then(|s| s.parse().ok())
                .filter(|&port| port > 0)
                .unwrap_or(4784),
            relay_multiaddr: self
                .get_string_setting(RELAY_MULTIADDR_KEY)?
                .filter(|s| !s.trim().is_empty()),
            p2p_announce: self
                .get_string_setting(P2P_ANNOUNCE_KEY)?
                .filter(|s| !s.trim().is_empty()),
            web_origin: self
                .get_string_setting(WEB_ORIGIN_KEY)?
                .filter(|s| !s.trim().is_empty()),
        })
    }

    pub fn set_feed_history_enabled(&self, enabled: bool) -> CoreResult<()> {
        self.set_string_setting(
            FEED_HISTORY_KEY,
            if enabled { "true" } else { "false" },
        )
    }

    pub fn update_connection_settings(
        &self,
        p2p_listen_port: Option<u16>,
        relay_multiaddr: Option<Option<String>>,
        p2p_announce: Option<Option<String>>,
        web_origin: Option<Option<String>>,
    ) -> CoreResult<()> {
        if let Some(port) = p2p_listen_port.filter(|&p| p > 0) {
            self.set_string_setting(P2P_LISTEN_PORT_KEY, &port.to_string())?;
        }
        if let Some(relay) = relay_multiaddr {
            match relay.filter(|s| !s.trim().is_empty()) {
                Some(value) => self.set_string_setting(RELAY_MULTIADDR_KEY, value.trim())?,
                None => self.delete_setting(RELAY_MULTIADDR_KEY)?,
            }
        }
        if let Some(announce) = p2p_announce {
            match announce.filter(|s| !s.trim().is_empty()) {
                Some(value) => self.set_string_setting(P2P_ANNOUNCE_KEY, value.trim())?,
                None => self.delete_setting(P2P_ANNOUNCE_KEY)?,
            }
        }
        if let Some(origin) = web_origin {
            match origin.filter(|s| !s.trim().is_empty()) {
                Some(value) => self.set_string_setting(WEB_ORIGIN_KEY, value.trim())?,
                None => self.delete_setting(WEB_ORIGIN_KEY)?,
            }
        }
        Ok(())
    }

    pub(super) fn get_bool_setting(&self, key: &str) -> CoreResult<Option<bool>> {
        Ok(self
            .get_string_setting(key)?
            .map(|value| value == "true"))
    }

    fn get_string_setting(&self, key: &str) -> CoreResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get::<_, String>("value")?))
        } else {
            Ok(None)
        }
    }

    fn set_string_setting(&self, key: &str, value: &str) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    fn delete_setting(&self, key: &str) -> CoreResult<()> {
        self.conn.execute("DELETE FROM app_settings WHERE key = ?1", params![key])?;
        Ok(())
    }
}
