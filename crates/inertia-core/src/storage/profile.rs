use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::error::{CoreError, CoreResult};

use super::{ProfileComment, ProfileItem, ProfilePhoto, Store};

impl Store {
    pub fn insert_profile_item(&self, item: &ProfileItem) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO profile_items (id, blob_hash, caption, content_id, sort_order, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                item.id,
                item.blob_hash,
                item.caption,
                item.content_id,
                item.sort_order,
                item.created_at.to_rfc3339(),
            ],
        )?;
        // Keep legacy table in sync for older readers during transition.
        let _ = self.insert_profile_photo(item);
        Ok(())
    }

    pub fn update_profile_item_content_id(
        &self,
        item_id: &str,
        content_id: &str,
    ) -> CoreResult<()> {
        self.conn.execute(
            "UPDATE profile_items SET content_id = ?1 WHERE id = ?2",
            params![content_id, item_id],
        )?;
        let _ = self.update_profile_photo_content_id(item_id, content_id);
        Ok(())
    }

    pub fn list_profile_items(&self) -> CoreResult<Vec<ProfileItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, blob_hash, caption, content_id, sort_order, created_at
             FROM profile_items ORDER BY sort_order, created_at DESC",
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
        let items = rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)?;
        if !items.is_empty() {
            return Ok(items);
        }
        // Fallback if migration has not run yet on a weird DB.
        self.list_profile_photos()
    }

    pub fn get_profile_item(&self, item_id: &str) -> CoreResult<Option<ProfileItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, blob_hash, caption, content_id, sort_order, created_at
             FROM profile_items WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![item_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ProfilePhoto {
                id: row.get("id")?,
                blob_hash: row.get("blob_hash")?,
                caption: row.get("caption")?,
                content_id: row.get("content_id")?,
                sort_order: row.get("sort_order")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn profile_item_blob_hashes(&self) -> CoreResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT blob_hash FROM profile_items")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn insert_profile_comment(&self, comment: &ProfileComment) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO profile_comments
             (id, profile_item_id, author_id, author_name, body, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                comment.id,
                comment.profile_item_id,
                comment.author_id,
                comment.author_name,
                comment.body,
                comment.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_profile_comments(&self, profile_item_id: &str) -> CoreResult<Vec<ProfileComment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, profile_item_id, author_id, author_name, body, created_at
             FROM profile_comments WHERE profile_item_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([profile_item_id], |row| {
            Ok(ProfileComment {
                id: row.get("id")?,
                profile_item_id: row.get("profile_item_id")?,
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

    pub fn count_profile_comments(&self, profile_item_id: &str) -> CoreResult<u32> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM profile_comments WHERE profile_item_id = ?1",
            params![profile_item_id],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }

    pub fn delete_profile_comment(&self, comment_id: &str) -> CoreResult<()> {
        self.conn.execute(
            "DELETE FROM profile_comments WHERE id = ?1",
            params![comment_id],
        )?;
        Ok(())
    }
}
