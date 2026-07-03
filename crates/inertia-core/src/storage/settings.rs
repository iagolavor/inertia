use rusqlite::params;

use crate::error::CoreResult;

use super::sql::{
    decode_multiaddrs, encode_multiaddrs, FEED_HISTORY_KEY, P2P_ANNOUNCE_KEY,
    P2P_LISTEN_PORT_KEY, RELAY_MULTIADDR_LEGACY_KEY, RELAY_MULTIADDRS_KEY, WEB_ORIGIN_KEY,
};
use super::{AppSettings, Store};

impl Store {
    pub fn get_settings(&self) -> CoreResult<AppSettings> {
        let mut relay_multiaddrs = self.get_relay_multiaddrs()?;
        if relay_multiaddrs.is_empty() {
            if let Some(legacy) = self
                .get_string_setting(RELAY_MULTIADDR_LEGACY_KEY)?
                .filter(|s| !s.trim().is_empty())
            {
                relay_multiaddrs = vec![legacy.trim().to_string()];
                self.set_relay_multiaddrs(&relay_multiaddrs)?;
                self.delete_setting(RELAY_MULTIADDR_LEGACY_KEY)?;
            }
        }

        Ok(AppSettings {
            feed_history_enabled: self.get_bool_setting(FEED_HISTORY_KEY)?.unwrap_or(false),
            p2p_listen_port: self
                .get_string_setting(P2P_LISTEN_PORT_KEY)?
                .and_then(|s| s.parse().ok())
                .filter(|&port| port > 0)
                .unwrap_or(4784),
            relay_multiaddrs,
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
        relay_multiaddrs: Option<Vec<String>>,
        p2p_announce: Option<Option<String>>,
        web_origin: Option<Option<String>>,
    ) -> CoreResult<()> {
        if let Some(port) = p2p_listen_port.filter(|&p| p > 0) {
            self.set_string_setting(P2P_LISTEN_PORT_KEY, &port.to_string())?;
        }
        if let Some(relays) = relay_multiaddrs {
            self.set_relay_multiaddrs(&relays)?;
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

    fn get_relay_multiaddrs(&self) -> CoreResult<Vec<String>> {
        Ok(self
            .get_string_setting(RELAY_MULTIADDRS_KEY)?
            .map(|raw| decode_multiaddrs(&raw))
            .unwrap_or_default())
    }

    fn set_relay_multiaddrs(&self, relays: &[String]) -> CoreResult<()> {
        let trimmed: Vec<String> = relays
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if trimmed.is_empty() {
            self.delete_setting(RELAY_MULTIADDRS_KEY)?;
        } else {
            self.set_string_setting(RELAY_MULTIADDRS_KEY, &encode_multiaddrs(&trimmed))?;
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

    #[cfg(test)]
    pub(crate) fn insert_setting_for_test(&self, key: &str, value: &str) -> CoreResult<()> {
        self.set_string_setting(key, value)
    }

    #[cfg(test)]
    pub(crate) fn has_setting(&self, key: &str) -> CoreResult<bool> {
        Ok(self.get_string_setting(key)?.is_some())
    }
}

#[cfg(test)]
mod settings_tests {
    use super::Store;

    const RELAY_MULTIADDR_LEGACY_KEY: &str = "relay_multiaddr";
    const RELAY_MULTIADDRS_KEY: &str = "relay_multiaddrs";
    const RELAY: &str =
        "/ip4/203.0.113.1/tcp/9000/p2p/12D3KooWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    #[test]
    fn legacy_single_relay_migrates_to_relay_list() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store
            .insert_setting_for_test(RELAY_MULTIADDR_LEGACY_KEY, RELAY)
            .unwrap();

        let settings = store.get_settings().unwrap();
        assert_eq!(settings.relay_multiaddrs, vec![RELAY.to_string()]);
        assert!(store.has_setting(RELAY_MULTIADDRS_KEY).unwrap());
        assert!(!store.has_setting(RELAY_MULTIADDR_LEGACY_KEY).unwrap());
    }
}
