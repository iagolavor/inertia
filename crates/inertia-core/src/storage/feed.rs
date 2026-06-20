use std::collections::HashMap;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::content::ContentType;
use crate::error::{CoreError, CoreResult};

use super::{
    ArchivedFeedItem, FeedBackup, FeedRestoreReport, InboxEntry, LocalPost, PostComment,
    ProfilePhoto, Store,
};

impl Store {
    pub fn insert_local_post(&self, post: &LocalPost) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO local_posts (content_id, body, media_ref, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                post.content_id,
                post.body,
                post.media_ref,
                post.created_at.to_rfc3339(),
                post.expires_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_local_posts(&self) -> CoreResult<Vec<LocalPost>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, body, media_ref, created_at, expires_at
             FROM local_posts ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(LocalPost {
                content_id: row.get("content_id")?,
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn list_inbox_posts(&self) -> CoreResult<Vec<InboxEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, sender_id, received_at, expires_at, read_at, body, media_ref, content_type
             FROM inbox WHERE content_type = 'post' ORDER BY received_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(InboxEntry {
                content_id: row.get("content_id")?,
                sender_id: row.get("sender_id")?,
                received_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("received_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                read_at: row
                    .get::<_, Option<String>>("read_at")?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                content_type: ContentType::Post,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn insert_profile_photo(&self, photo: &ProfilePhoto) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO profile_photos (id, blob_hash, caption, content_id, sort_order, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                photo.id,
                photo.blob_hash,
                photo.caption,
                photo.content_id,
                photo.sort_order,
                photo.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn update_profile_photo_content_id(
        &self,
        photo_id: &str,
        content_id: &str,
    ) -> CoreResult<()> {
        self.conn.execute(
            "UPDATE profile_photos SET content_id = ?1 WHERE id = ?2",
            params![content_id, photo_id],
        )?;
        Ok(())
    }

    pub fn list_profile_photos(&self) -> CoreResult<Vec<ProfilePhoto>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, blob_hash, caption, content_id, sort_order, created_at
             FROM profile_photos ORDER BY sort_order, created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProfilePhoto {
                id: row.get("id")?,
                blob_hash: row.get("blob_hash")?,
                caption: row.get("caption")?,
                content_id: row.get("content_id")?,
                sort_order: row.get("sort_order")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn insert_post_comment(&self, comment: &PostComment) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO post_comments
             (id, post_id, author_id, author_name, body, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                comment.id,
                comment.post_id,
                comment.author_id,
                comment.author_name,
                comment.body,
                comment.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_post_comments(&self, post_id: &str) -> CoreResult<Vec<PostComment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, post_id, author_id, author_name, body, created_at
             FROM post_comments WHERE post_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([post_id], |row| {
            Ok(PostComment {
                id: row.get("id")?,
                post_id: row.get("post_id")?,
                author_id: row.get("author_id")?,
                author_name: row.get("author_name")?,
                body: row.get("body")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn count_post_comments(&self, post_id: &str) -> CoreResult<u32> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM post_comments WHERE post_id = ?1",
            params![post_id],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }

    pub fn get_local_post(&self, content_id: &str) -> CoreResult<Option<LocalPost>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, body, media_ref, created_at, expires_at
             FROM local_posts WHERE content_id = ?1",
        )?;
        let mut rows = stmt.query([content_id])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(LocalPost {
                content_id: row.get("content_id")?,
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            }));
        }
        Ok(None)
    }

    pub fn upsert_feed_archive(&self, item: &ArchivedFeedItem) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO feed_archive
             (content_id, author_id, author_name, body, media_ref, created_at, is_own)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(content_id) DO UPDATE SET
               author_id = excluded.author_id,
               author_name = excluded.author_name,
               body = excluded.body,
               media_ref = excluded.media_ref,
               created_at = excluded.created_at,
               is_own = excluded.is_own",
            params![
                item.content_id,
                item.author_id,
                item.author_name,
                item.body,
                item.media_ref,
                item.created_at.to_rfc3339(),
                item.is_own as i32,
            ],
        )?;
        Ok(())
    }

    pub fn try_archive_feed_item(&self, item: &ArchivedFeedItem) -> CoreResult<()> {
        if self.get_settings()?.feed_history_enabled {
            self.upsert_feed_archive(item)?;
        }
        Ok(())
    }

    pub fn list_feed_archive(&self) -> CoreResult<Vec<ArchivedFeedItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, author_id, author_name, body, media_ref, created_at, is_own
             FROM feed_archive ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ArchivedFeedItem {
                content_id: row.get("content_id")?,
                author_id: row.get("author_id")?,
                author_name: row.get("author_name")?,
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                is_own: row.get::<_, i32>("is_own")? != 0,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn export_feed_backup(&self) -> CoreResult<FeedBackup> {
        let items = self.list_feed_archive()?;
        let mut blobs = HashMap::new();
        for item in &items {
            if let Some(ref hash) = item.media_ref {
                if blobs.contains_key(hash) {
                    continue;
                }
                if let Ok(data) = self.read_blob(hash) {
                    blobs.insert(hash.clone(), BASE64.encode(data));
                }
            }
        }
        Ok(FeedBackup {
            version: 1,
            exported_at: Utc::now(),
            items,
            blobs,
        })
    }

    pub fn import_feed_backup(&self, backup: &FeedBackup) -> CoreResult<FeedRestoreReport> {
        if backup.version != 1 {
            return Err(CoreError::Invite("unsupported backup version".into()));
        }

        let mut blobs_imported = 0usize;
        for (hash, b64) in &backup.blobs {
            let path = self.blob_path(hash);
            if path.exists() {
                continue;
            }
            let data = BASE64.decode(b64).map_err(|e| CoreError::Crypto(e.to_string()))?;
            std::fs::write(&path, data)?;
            blobs_imported += 1;
        }

        let mut items_imported = 0usize;
        for item in &backup.items {
            let existed = self.conn.query_row(
                "SELECT 1 FROM feed_archive WHERE content_id = ?1",
                params![item.content_id],
                |_| Ok(()),
            );
            if existed.is_ok() {
                continue;
            }
            self.upsert_feed_archive(item)?;
            items_imported += 1;
        }

        Ok(FeedRestoreReport {
            items_imported,
            blobs_imported,
        })
    }
}
