use rusqlite::params;

use crate::error::{CoreError, CoreResult};
use crate::identity::{encode_hex, Identity};

use super::Store;

impl Store {
    pub fn update_identity_avatar(&self, avatar_blob_hash: &str) -> CoreResult<()> {
        let updated = self.conn.execute(
            "UPDATE identity SET avatar_blob_hash = ?1 WHERE id = 1",
            params![avatar_blob_hash],
        )?;
        if updated == 0 {
            return Err(CoreError::IdentityNotInitialized);
        }
        Ok(())
    }

    pub fn update_identity_profile(
        &self,
        display_name: &str,
        bio: &str,
    ) -> CoreResult<()> {
        if display_name.trim().is_empty() {
            return Err(CoreError::IdentityNotInitialized);
        }
        let updated = self.conn.execute(
            "UPDATE identity SET display_name = ?1, bio = ?2 WHERE id = 1",
            params![display_name.trim(), bio.trim()],
        )?;
        if updated == 0 {
            return Err(CoreError::IdentityNotInitialized);
        }
        Ok(())
    }

    pub fn has_profile(&self) -> CoreResult<bool> {
        Ok(self.load_identity()?.is_some())
    }

    pub fn create_identity(&self, identity: &Identity) -> CoreResult<()> {
        if self.has_profile()? {
            return Err(CoreError::ProfileAlreadyExists);
        }

        let signing_key = identity.signing_key()?.to_bytes();
        let encryption_secret = identity.encryption_secret()?.to_bytes();

        self.conn
            .execute(
                "INSERT INTO identity
                 (id, signing_pubkey, encryption_pubkey, phone_hash, display_name, bio, signing_key, encryption_secret)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    identity.signing_pubkey,
                    identity.encryption_pubkey,
                    identity.phone_hash,
                    identity.display_name,
                    identity.bio,
                    encode_hex(signing_key),
                    encode_hex(encryption_secret),
                ],
            )
            .map_err(|e| {
                if matches!(
                    e,
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error {
                            code: rusqlite::ErrorCode::ConstraintViolation,
                            ..
                        },
                        _
                    )
                ) {
                    CoreError::ProfileAlreadyExists
                } else {
                    CoreError::Database(e)
                }
            })?;
        Ok(())
    }

    pub fn load_identity(&self) -> CoreResult<Option<Identity>> {
        let mut stmt = self.conn.prepare(
            "SELECT signing_pubkey, encryption_pubkey, phone_hash, display_name, bio, avatar_blob_hash, signing_key, encryption_secret
             FROM identity WHERE id = 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let identity = Identity::from_persisted(
                row.get("signing_pubkey")?,
                row.get("encryption_pubkey")?,
                row.get("phone_hash")?,
                row.get("display_name")?,
                row.get("bio").unwrap_or_default(),
                row.get("avatar_blob_hash").ok(),
                row.get("signing_key")?,
                row.get("encryption_secret")?,
            )?;
            if identity.is_initialized() {
                return Ok(Some(identity));
            }
        }
        Ok(None)
    }
}
