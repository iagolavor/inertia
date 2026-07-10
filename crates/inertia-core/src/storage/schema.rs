use crate::error::CoreResult;

use super::Store;

impl Store {
    pub(super) fn migrate(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS identity (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                signing_pubkey TEXT NOT NULL,
                encryption_pubkey TEXT NOT NULL,
                phone_hash TEXT,
                display_name TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS contacts (
                id TEXT PRIMARY KEY,
                phone_hash TEXT,
                display_name TEXT NOT NULL,
                peer_id TEXT,
                signing_pubkey TEXT NOT NULL,
                encryption_pubkey TEXT NOT NULL,
                last_seen TEXT,
                connection_state TEXT NOT NULL DEFAULT 'offline'
            );

            CREATE TABLE IF NOT EXISTS outbox (
                content_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                status TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                ciphertext BLOB NOT NULL,
                content_type TEXT NOT NULL,
                envelope_json TEXT NOT NULL,
                PRIMARY KEY (content_id, recipient_id)
            );

            CREATE TABLE IF NOT EXISTS inbox (
                content_id TEXT PRIMARY KEY,
                sender_id TEXT NOT NULL,
                received_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                read_at TEXT,
                body TEXT NOT NULL,
                content_type TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS delivery_acks (
                content_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                acked_at TEXT NOT NULL,
                PRIMARY KEY (content_id, recipient_id)
            );

            CREATE TABLE IF NOT EXISTS issued_invites (
                nonce TEXT PRIMARY KEY,
                expires_at TEXT NOT NULL,
                consumed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS redeemed_invites (
                nonce TEXT PRIMARY KEY,
                issuer_signing_pubkey TEXT NOT NULL,
                redeemed_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS local_posts (
                content_id TEXT PRIMARY KEY,
                body TEXT NOT NULL,
                media_ref TEXT,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS profile_photos (
                id TEXT PRIMARY KEY,
                blob_hash TEXT NOT NULL,
                caption TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            ",
        )?;
        self.ensure_identity_key_columns()?;
        self.ensure_identity_bio_column()?;
        self.ensure_identity_avatar_column()?;
        self.ensure_inbox_media_ref_column()?;
        self.ensure_feed_archive_tables()?;
        self.ensure_post_comments_table()?;
        self.ensure_profile_photo_content_id_column()?;
        self.ensure_contact_multiaddrs_column()?;
        self.ensure_sent_messages_table()?;
        self.ensure_media_manifests_table()?;
        self.ensure_profile_items_table()?;
        self.ensure_profile_comments_table()?;
        self.ensure_archive_tables()?;
        self.ensure_archive_uploads_table()?;
        Ok(())
    }

    fn ensure_archive_uploads_table(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS archive_uploads (
                id TEXT PRIMARY KEY,
                folder_id TEXT NOT NULL,
                name TEXT NOT NULL,
                mime TEXT NOT NULL,
                total_bytes INTEGER NOT NULL,
                chunk_size INTEGER NOT NULL,
                chunks_total INTEGER NOT NULL,
                root_hash TEXT,
                created_at TEXT NOT NULL,
                completed_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_archive_uploads_folder
                ON archive_uploads(folder_id);
            ",
        )?;
        Ok(())
    }

    fn ensure_profile_items_table(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS profile_items (
                id TEXT PRIMARY KEY,
                blob_hash TEXT NOT NULL,
                caption TEXT,
                content_id TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_profile_items_sort
                ON profile_items(sort_order, created_at);
            ",
        )?;
        // One-time copy from legacy profile_photos (idempotent via INSERT OR IGNORE).
        self.conn.execute_batch(
            "
            INSERT OR IGNORE INTO profile_items
                (id, blob_hash, caption, content_id, sort_order, created_at)
            SELECT id, blob_hash, caption, content_id, sort_order, created_at
            FROM profile_photos;
            ",
        )?;
        Ok(())
    }

    fn ensure_profile_comments_table(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS profile_comments (
                id TEXT PRIMARY KEY,
                profile_item_id TEXT NOT NULL,
                author_id TEXT NOT NULL,
                author_name TEXT NOT NULL,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_profile_comments_item
                ON profile_comments(profile_item_id, created_at);
            ",
        )?;
        Ok(())
    }

    fn ensure_archive_tables(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS archive_folders (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS archive_entries (
                id TEXT PRIMARY KEY,
                folder_id TEXT NOT NULL,
                name TEXT NOT NULL,
                root_hash TEXT NOT NULL,
                total_bytes INTEGER NOT NULL DEFAULT 0,
                mime TEXT NOT NULL DEFAULT 'application/octet-stream',
                created_at TEXT NOT NULL,
                FOREIGN KEY (folder_id) REFERENCES archive_folders(id)
            );
            CREATE INDEX IF NOT EXISTS idx_archive_entries_folder
                ON archive_entries(folder_id, created_at);
            ",
        )?;
        Ok(())
    }

    fn ensure_media_manifests_table(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS media_manifests (
                root_hash TEXT PRIMARY KEY,
                manifest_json TEXT NOT NULL,
                expires_at TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    fn ensure_sent_messages_table(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sent_messages (
                content_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                body TEXT NOT NULL,
                sent_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                status TEXT NOT NULL,
                PRIMARY KEY (content_id, recipient_id)
            );
            CREATE INDEX IF NOT EXISTS idx_sent_messages_recipient
                ON sent_messages(recipient_id);
            ",
        )?;
        Ok(())
    }

    fn ensure_contact_multiaddrs_column(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(contacts)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();
        if !cols.iter().any(|c| c == "multiaddrs") {
            self.conn.execute(
                "ALTER TABLE contacts ADD COLUMN multiaddrs TEXT NOT NULL DEFAULT '[]'",
                [],
            )?;
        }
        Ok(())
    }

    fn ensure_post_comments_table(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS post_comments (
                id TEXT PRIMARY KEY,
                post_id TEXT NOT NULL,
                author_id TEXT NOT NULL,
                author_name TEXT NOT NULL,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_post_comments_post_id ON post_comments(post_id);
            ",
        )?;
        Ok(())
    }

    fn ensure_profile_photo_content_id_column(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(profile_photos)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();

        if !cols.iter().any(|c| c == "content_id") {
            self.conn
                .execute("ALTER TABLE profile_photos ADD COLUMN content_id TEXT", [])?;
        }
        Ok(())
    }

    fn ensure_feed_archive_tables(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS feed_archive (
                content_id TEXT PRIMARY KEY,
                author_id TEXT NOT NULL,
                author_name TEXT NOT NULL,
                body TEXT NOT NULL,
                media_ref TEXT,
                created_at TEXT NOT NULL,
                is_own INTEGER NOT NULL DEFAULT 0
            );
            ",
        )?;
        Ok(())
    }

    fn ensure_inbox_media_ref_column(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(inbox)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();

        if !cols.iter().any(|c| c == "media_ref") {
            self.conn
                .execute("ALTER TABLE inbox ADD COLUMN media_ref TEXT", [])?;
        }
        Ok(())
    }

    fn ensure_identity_key_columns(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(identity)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();

        if !cols.iter().any(|c| c == "signing_key") {
            self.conn
                .execute("ALTER TABLE identity ADD COLUMN signing_key TEXT", [])?;
        }
        if !cols.iter().any(|c| c == "encryption_secret") {
            self.conn
                .execute("ALTER TABLE identity ADD COLUMN encryption_secret TEXT", [])?;
        }
        Ok(())
    }

    fn ensure_identity_bio_column(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(identity)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();

        if !cols.iter().any(|c| c == "bio") {
            self.conn
                .execute("ALTER TABLE identity ADD COLUMN bio TEXT NOT NULL DEFAULT ''", [])?;
        }
        Ok(())
    }

    fn ensure_identity_avatar_column(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(identity)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();

        if !cols.iter().any(|c| c == "avatar_blob_hash") {
            self.conn
                .execute("ALTER TABLE identity ADD COLUMN avatar_blob_hash TEXT", [])?;
        }
        Ok(())
    }
}
