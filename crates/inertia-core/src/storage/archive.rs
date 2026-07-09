use chrono::{DateTime, Utc};
use rusqlite::params;
use std::path::PathBuf;

use crate::error::{CoreError, CoreResult};

use super::{
    ArchiveEntry, ArchiveFolder, ArchiveFolderSummary, ArchiveUpload, ArchiveUploadStatus, Store,
};

impl Store {
    pub fn insert_archive_folder(&self, folder: &ArchiveFolder) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO archive_folders (id, name, created_at) VALUES (?1, ?2, ?3)",
            params![folder.id, folder.name, folder.created_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn list_archive_folders(&self) -> CoreResult<Vec<ArchiveFolder>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, created_at FROM archive_folders ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ArchiveFolder {
                id: row.get("id")?,
                name: row.get("name")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn get_archive_folder(&self, folder_id: &str) -> CoreResult<Option<ArchiveFolder>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, created_at FROM archive_folders WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![folder_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ArchiveFolder {
                id: row.get("id")?,
                name: row.get("name")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_archive_folder(&self, folder_id: &str) -> CoreResult<()> {
        self.conn.execute(
            "DELETE FROM archive_entries WHERE folder_id = ?1",
            params![folder_id],
        )?;
        self.conn.execute(
            "DELETE FROM archive_folders WHERE id = ?1",
            params![folder_id],
        )?;
        Ok(())
    }

    pub fn insert_archive_entry(&self, entry: &ArchiveEntry) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO archive_entries
             (id, folder_id, name, root_hash, total_bytes, mime, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id,
                entry.folder_id,
                entry.name,
                entry.root_hash,
                entry.total_bytes as i64,
                entry.mime,
                entry.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_archive_entries(&self, folder_id: &str) -> CoreResult<Vec<ArchiveEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, folder_id, name, root_hash, total_bytes, mime, created_at
             FROM archive_entries WHERE folder_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([folder_id], |row| {
            Ok(ArchiveEntry {
                id: row.get("id")?,
                folder_id: row.get("folder_id")?,
                name: row.get("name")?,
                root_hash: row.get("root_hash")?,
                total_bytes: row.get::<_, i64>("total_bytes")? as u64,
                mime: row.get("mime")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn get_archive_entry(&self, entry_id: &str) -> CoreResult<Option<ArchiveEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, folder_id, name, root_hash, total_bytes, mime, created_at
             FROM archive_entries WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![entry_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ArchiveEntry {
                id: row.get("id")?,
                folder_id: row.get("folder_id")?,
                name: row.get("name")?,
                root_hash: row.get("root_hash")?,
                total_bytes: row.get::<_, i64>("total_bytes")? as u64,
                mime: row.get("mime")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_archive_entry_by_root_hash(
        &self,
        root_hash: &str,
    ) -> CoreResult<Option<ArchiveEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, folder_id, name, root_hash, total_bytes, mime, created_at
             FROM archive_entries WHERE root_hash = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query(params![root_hash])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ArchiveEntry {
                id: row.get("id")?,
                folder_id: row.get("folder_id")?,
                name: row.get("name")?,
                root_hash: row.get("root_hash")?,
                total_bytes: row.get::<_, i64>("total_bytes")? as u64,
                mime: row.get("mime")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_archive_entry(&self, entry_id: &str) -> CoreResult<()> {
        self.conn.execute(
            "DELETE FROM archive_entries WHERE id = ?1",
            params![entry_id],
        )?;
        Ok(())
    }

    pub fn list_archive_folder_summaries(&self) -> CoreResult<Vec<ArchiveFolderSummary>> {
        let folders = self.list_archive_folders()?;
        let mut out = Vec::with_capacity(folders.len());
        for folder in folders {
            let count: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM archive_entries WHERE folder_id = ?1",
                params![folder.id],
                |row| row.get(0),
            )?;
            out.push(ArchiveFolderSummary {
                id: folder.id,
                name: folder.name,
                entry_count: count as u32,
                created_at: folder.created_at,
            });
        }
        Ok(out)
    }

    pub fn archive_entry_root_hashes(&self) -> CoreResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT root_hash FROM archive_entries")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn insert_archive_upload(&self, upload: &ArchiveUpload) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO archive_uploads
             (id, folder_id, name, mime, total_bytes, chunk_size, chunks_total, root_hash, created_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                upload.id,
                upload.folder_id,
                upload.name,
                upload.mime,
                upload.total_bytes as i64,
                upload.chunk_size as i64,
                upload.chunks_total as i64,
                upload.root_hash,
                upload.created_at.to_rfc3339(),
                upload.completed_at.map(|t| t.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn get_archive_upload(&self, upload_id: &str) -> CoreResult<Option<ArchiveUpload>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, folder_id, name, mime, total_bytes, chunk_size, chunks_total, root_hash, created_at, completed_at
             FROM archive_uploads WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![upload_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ArchiveUpload {
                id: row.get("id")?,
                folder_id: row.get("folder_id")?,
                name: row.get("name")?,
                mime: row.get("mime")?,
                total_bytes: row.get::<_, i64>("total_bytes")? as u64,
                chunk_size: row.get::<_, i64>("chunk_size")? as u32,
                chunks_total: row.get::<_, i64>("chunks_total")? as u32,
                root_hash: row.get("root_hash")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                completed_at: row
                    .get::<_, Option<String>>("completed_at")?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn set_archive_upload_root_hash(&self, upload_id: &str, root_hash: &str) -> CoreResult<()> {
        self.conn.execute(
            "UPDATE archive_uploads SET root_hash = ?1 WHERE id = ?2",
            params![root_hash, upload_id],
        )?;
        Ok(())
    }

    pub fn mark_archive_upload_complete(&self, upload_id: &str) -> CoreResult<()> {
        self.conn.execute(
            "UPDATE archive_uploads SET completed_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), upload_id],
        )?;
        Ok(())
    }

    /// Staging dir for chunk hashes during ingest (before root_hash is known).
    pub fn upload_chunk_dir(&self, upload_id: &str) -> PathBuf {
        self.blob_dir.join("uploads").join(upload_id)
    }

    pub fn upload_chunk_path(&self, upload_id: &str, index: u32) -> PathBuf {
        self.upload_chunk_dir(upload_id).join(format!("{index:05}"))
    }

    pub fn store_upload_chunk(
        &self,
        upload_id: &str,
        index: u32,
        expected_hash: &str,
        data: &[u8],
    ) -> CoreResult<()> {
        use sha2::{Digest, Sha256};
        use crate::identity::encode_hex;

        if data.len() > super::CHUNK_SIZE {
            return Err(CoreError::P2p(format!(
                "chunk too large ({} bytes, max {})",
                data.len(),
                super::CHUNK_SIZE
            )));
        }
        let computed = encode_hex(Sha256::digest(data));
        if computed != expected_hash {
            return Err(CoreError::Crypto("chunk hash mismatch".into()));
        }
        let dir = self.upload_chunk_dir(upload_id);
        std::fs::create_dir_all(&dir)?;
        let path = self.upload_chunk_path(upload_id, index);
        if path.exists() {
            // Idempotent resume: already stored.
            return Ok(());
        }
        std::fs::write(path, data)?;
        // Also write sidecar hash for complete step.
        std::fs::write(
            self.upload_chunk_dir(upload_id).join(format!("{index:05}.sha")),
            expected_hash.as_bytes(),
        )?;
        Ok(())
    }

    pub fn upload_chunk_exists(&self, upload_id: &str, index: u32) -> bool {
        self.upload_chunk_path(upload_id, index).exists()
    }

    pub fn count_upload_chunks(&self, upload_id: &str, chunks_total: u32) -> u32 {
        (0..chunks_total)
            .filter(|i| self.upload_chunk_exists(upload_id, *i))
            .count() as u32
    }

    pub fn missing_upload_chunks(&self, upload_id: &str, chunks_total: u32) -> Vec<u32> {
        (0..chunks_total)
            .filter(|i| !self.upload_chunk_exists(upload_id, *i))
            .collect()
    }

    pub fn read_upload_chunk_hash(&self, upload_id: &str, index: u32) -> CoreResult<String> {
        let path = self.upload_chunk_dir(upload_id).join(format!("{index:05}.sha"));
        if !path.exists() {
            return Err(CoreError::ContentNotFound(format!(
                "upload chunk hash {upload_id}#{index}"
            )));
        }
        Ok(String::from_utf8_lossy(&std::fs::read(path)?).to_string())
    }

    pub fn archive_upload_status(&self, upload_id: &str) -> CoreResult<Option<ArchiveUploadStatus>> {
        let Some(upload) = self.get_archive_upload(upload_id)? else {
            return Ok(None);
        };
        let missing = self.missing_upload_chunks(upload_id, upload.chunks_total);
        let chunks_done = upload.chunks_total.saturating_sub(missing.len() as u32);
        Ok(Some(ArchiveUploadStatus {
            upload_id: upload.id,
            folder_id: upload.folder_id,
            name: upload.name,
            mime: upload.mime,
            total_bytes: upload.total_bytes,
            chunk_size: upload.chunk_size,
            chunks_done,
            chunks_total: upload.chunks_total,
            missing,
            completed: upload.completed_at.is_some(),
        }))
    }

    pub fn finalize_archive_upload(&self, upload_id: &str) -> CoreResult<ArchiveEntry> {
        use crate::content::{MediaKind, MediaManifest};
        use crate::storage::media::{hash_manifest_body, ManifestBody};

        let upload = self
            .get_archive_upload(upload_id)?
            .ok_or_else(|| CoreError::ContentNotFound(upload_id.to_string()))?;
        if upload.completed_at.is_some() {
            if let Some(root) = &upload.root_hash {
                if let Some(entry) = self.get_archive_entry_by_root_hash(root)? {
                    return Ok(entry);
                }
            }
            return Err(CoreError::Invite("upload completed but entry missing".into()));
        }

        let missing = self.missing_upload_chunks(upload_id, upload.chunks_total);
        if !missing.is_empty() {
            return Err(CoreError::Invite(format!(
                "missing {} chunks before complete",
                missing.len()
            )));
        }

        let mut chunk_hashes = Vec::with_capacity(upload.chunks_total as usize);
        for i in 0..upload.chunks_total {
            chunk_hashes.push(self.read_upload_chunk_hash(upload_id, i)?);
        }

        let body = ManifestBody {
            kind: MediaKind::File,
            mime: upload.mime.clone(),
            total_bytes: upload.total_bytes,
            chunk_size: upload.chunk_size,
            chunk_hashes: chunk_hashes.clone(),
            thumb_hash: String::new(),
            duration_ms: 0,
        };
        let root_hash = hash_manifest_body(&body);
        let manifest = MediaManifest {
            root_hash: root_hash.clone(),
            kind: MediaKind::File,
            mime: body.mime.clone(),
            total_bytes: body.total_bytes,
            chunk_size: body.chunk_size,
            chunk_hashes: body.chunk_hashes.clone(),
            thumb_hash: body.thumb_hash.clone(),
            duration_ms: 0,
        };

        for i in 0..upload.chunks_total {
            let data = std::fs::read(self.upload_chunk_path(upload_id, i))?;
            self.store_chunk_verified(&root_hash, i, &chunk_hashes[i as usize], &data)?;
        }
        self.insert_manifest_durable(&manifest)?;
        self.assemble_media_if_complete(&manifest)?;
        self.set_archive_upload_root_hash(upload_id, &root_hash)?;

        let entry = ArchiveEntry {
            id: uuid::Uuid::new_v4().to_string(),
            folder_id: upload.folder_id.clone(),
            name: upload.name.clone(),
            root_hash: root_hash.clone(),
            total_bytes: upload.total_bytes,
            mime: upload.mime.clone(),
            created_at: Utc::now(),
        };
        self.insert_archive_entry(&entry)?;
        self.mark_archive_upload_complete(upload_id)?;
        let _ = std::fs::remove_dir_all(self.upload_chunk_dir(upload_id));
        Ok(entry)
    }
}
