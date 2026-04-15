//! SQLite-backed session context cache.

use redberry_core::{ContextMessage, RedberryError, SessionContext};
use rusqlite::{params, Connection};
use std::path::Path;

/// The session context cache database.
pub struct ContextCache {
    conn: Connection,
}

impl ContextCache {
    /// Initialize the cache database at the given path.
    pub fn new(db_path: &Path) -> Result<Self, RedberryError> {
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let conn = Connection::open(db_path).map_err(|e| {
            RedberryError::Cache(format!(
                "Failed to open SQLite db at {}: {}",
                db_path.display(),
                e
            ))
        })?;

        // Initialize schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                consecutive_bad INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(|e| RedberryError::Cache(format!("Failed to create sessions table: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                session_id TEXT NOT NULL,
                idx INTEGER NOT NULL,
                text TEXT NOT NULL,
                embedding BLOB NOT NULL,
                snark_response TEXT,
                metrics_vagueness REAL NOT NULL DEFAULT 0.0,
                metrics_syntax REAL NOT NULL DEFAULT 0.0,
                metrics_drift REAL NOT NULL DEFAULT 0.0,
                created_at INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (session_id, idx),
                FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| RedberryError::Cache(format!("Failed to create messages table: {}", e)))?;

        // Add consecutive_bad column if migrating
        let _ = conn.execute(
            "ALTER TABLE sessions ADD COLUMN consecutive_bad INTEGER NOT NULL DEFAULT 0",
            [],
        );

        // Add metric columns if migrating from V1
        let _ = conn.execute("ALTER TABLE messages ADD COLUMN snark_response TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE messages ADD COLUMN metrics_vagueness REAL NOT NULL DEFAULT 0.0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE messages ADD COLUMN metrics_syntax REAL NOT NULL DEFAULT 0.0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE messages ADD COLUMN metrics_drift REAL NOT NULL DEFAULT 0.0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE messages ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0",
            [],
        );

        // Optional: Retroactively map timestamp to 0'd cols if migrating
        let _ = conn.execute("UPDATE messages SET created_at = CAST(strftime('%s', 'now') AS INTEGER) WHERE created_at = 0", []);

        Ok(Self { conn })
    }

    /// Retrieve the complete context for a session.
    pub fn get_context(&self, session_id: &str) -> Result<Option<SessionContext>, RedberryError> {
        let mut check_stmt = self
            .conn
            .prepare("SELECT consecutive_bad FROM sessions WHERE id = ?1")
            .map_err(|e| RedberryError::Cache(e.to_string()))?;

        let mut consecutive_bad = 0;
        let mut row_iter = check_stmt
            .query(params![session_id])
            .map_err(|e| RedberryError::Cache(e.to_string()))?;

        if let Some(row) = row_iter
            .next()
            .map_err(|e| RedberryError::Cache(e.to_string()))?
        {
            consecutive_bad = row.get(0).unwrap_or(0);
        } else {
            return Ok(None);
        }

        let mut msg_stmt = self.conn.prepare(
            "SELECT text, embedding, snark_response, metrics_vagueness, metrics_syntax, metrics_drift, created_at FROM messages WHERE session_id = ?1 ORDER BY idx ASC"
        ).map_err(|e| RedberryError::Cache(e.to_string()))?;

        let messages_iter = msg_stmt
            .query_map(params![session_id], |row| {
                let text: String = row.get(0)?;
                let embedding_bytes: Vec<u8> = row.get(1)?;
                let snark_response: Option<String> = row.get(2)?;
                let metrics_vagueness: f64 = row.get(3).unwrap_or(0.0);
                let metrics_syntax: f64 = row.get(4).unwrap_or(0.0);
                let metrics_drift: f64 = row.get(5).unwrap_or(0.0);
                let created_at: Option<i64> = row.get(6)?;

                // Deserialize floats from bytes
                let embedding: Vec<f32> = embedding_bytes
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                    .collect();

                Ok(ContextMessage {
                    text,
                    embedding,
                    snark_response,
                    metrics_vagueness: metrics_vagueness as f32,
                    metrics_syntax: metrics_syntax as f32,
                    metrics_drift: metrics_drift as f32,
                    created_at,
                })
            })
            .map_err(|e| RedberryError::Cache(e.to_string()))?;

        let mut messages = Vec::new();
        for msg in messages_iter {
            messages.push(msg.map_err(|e| RedberryError::Cache(e.to_string()))?);
        }

        Ok(Some(SessionContext {
            session_id: session_id.to_string(),
            messages,
            consecutive_bad,
        }))
    }

    /// Store or update a full session context.
    /// Overwrites any existing messages for the session.
    pub fn store_context(&mut self, context: &SessionContext) -> Result<(), RedberryError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let tx = self
            .conn
            .transaction()
            .map_err(|e| RedberryError::Cache(e.to_string()))?;

        // Upsert session
        tx.execute(
            "INSERT INTO sessions (id, created_at, updated_at, consecutive_bad) 
             VALUES (?1, ?2, ?3, ?4) 
             ON CONFLICT(id) DO UPDATE SET updated_at = ?3, consecutive_bad = ?4",
            params![context.session_id, now, now, context.consecutive_bad],
        )
        .map_err(|e| RedberryError::Cache(e.to_string()))?;

        // Delete old messages
        tx.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![context.session_id],
        )
        .map_err(|e| RedberryError::Cache(e.to_string()))?;

        // Insert new messages
        let mut msg_stmt = tx.prepare(
            "INSERT INTO messages (session_id, idx, text, embedding, snark_response, metrics_vagueness, metrics_syntax, metrics_drift, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        ).map_err(|e| RedberryError::Cache(e.to_string()))?;

        for (idx, msg) in context.messages.iter().enumerate() {
            // Serialize floats to bytes
            let mut embedding_bytes = Vec::with_capacity(msg.embedding.len() * 4);
            for &f in &msg.embedding {
                embedding_bytes.extend_from_slice(&f.to_le_bytes());
            }

            let timestamp = msg.created_at.unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
            });

            msg_stmt
                .execute(params![
                    context.session_id,
                    idx as i64,
                    msg.text,
                    embedding_bytes,
                    msg.snark_response,
                    msg.metrics_vagueness as f64,
                    msg.metrics_syntax as f64,
                    msg.metrics_drift as f64,
                    timestamp,
                ])
                .map_err(|e| RedberryError::Cache(e.to_string()))?;
        }

        // Drop the prepared statement before committing
        drop(msg_stmt);

        tx.commit()
            .map_err(|e| RedberryError::Cache(e.to_string()))?;
        Ok(())
    }

    /// Append new messages to an existing session. Creates session if it doesn't exist.
    pub fn append_messages(
        &mut self,
        session_id: &str,
        new_messages: &[ContextMessage],
        consecutive_bad: u32,
    ) -> Result<(), RedberryError> {
        if new_messages.is_empty() {
            return Ok(());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let tx = self
            .conn
            .transaction()
            .map_err(|e| RedberryError::Cache(e.to_string()))?;

        tx.execute(
            "INSERT INTO sessions (id, created_at, updated_at, consecutive_bad) 
             VALUES (?1, ?2, ?3, ?4) 
             ON CONFLICT(id) DO UPDATE SET updated_at = ?3, consecutive_bad = ?4",
            params![session_id, now, now, consecutive_bad],
        )
        .map_err(|e| RedberryError::Cache(e.to_string()))?;

        let current_count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let mut msg_stmt = tx.prepare(
            "INSERT INTO messages (session_id, idx, text, embedding, snark_response, metrics_vagueness, metrics_syntax, metrics_drift, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        ).map_err(|e| RedberryError::Cache(e.to_string()))?;

        for (i, msg) in new_messages.iter().enumerate() {
            let idx = current_count + i as i64;

            let mut embedding_bytes = Vec::with_capacity(msg.embedding.len() * 4);
            for &f in &msg.embedding {
                embedding_bytes.extend_from_slice(&f.to_le_bytes());
            }

            let timestamp = msg.created_at.unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
            });

            msg_stmt
                .execute(params![
                    session_id,
                    idx,
                    msg.text,
                    embedding_bytes,
                    msg.snark_response,
                    msg.metrics_vagueness as f64,
                    msg.metrics_syntax as f64,
                    msg.metrics_drift as f64,
                    timestamp,
                ])
                .map_err(|e| RedberryError::Cache(e.to_string()))?;
        }

        drop(msg_stmt);
        tx.commit()
            .map_err(|e| RedberryError::Cache(e.to_string()))?;
        Ok(())
    }

    /// Retrieve all stored messages globally for UI dashboards.
    pub fn get_all_messages(&self) -> Result<Vec<(String, ContextMessage)>, RedberryError> {
        let mut msg_stmt = self.conn.prepare(
            "SELECT session_id, text, embedding, snark_response, metrics_vagueness, metrics_syntax, metrics_drift, created_at 
             FROM messages ORDER BY session_id ASC, idx ASC"
        ).map_err(|e| RedberryError::Cache(e.to_string()))?;

        let messages_iter = msg_stmt
            .query_map([], |row| {
                let session_id: String = row.get(0)?;
                let text: String = row.get(1)?;
                let embedding_bytes: Vec<u8> = row.get(2)?;
                let snark_response: Option<String> = row.get(3)?;
                let metrics_vagueness: f64 = row.get(4).unwrap_or(0.0);
                let metrics_syntax: f64 = row.get(5).unwrap_or(0.0);
                let metrics_drift: f64 = row.get(6).unwrap_or(0.0);
                let created_at: Option<i64> = row.get(7)?;

                let embedding: Vec<f32> = embedding_bytes
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                    .collect();

                Ok((
                    session_id,
                    ContextMessage {
                        text,
                        embedding,
                        snark_response,
                        metrics_vagueness: metrics_vagueness as f32,
                        metrics_syntax: metrics_syntax as f32,
                        metrics_drift: metrics_drift as f32,
                        created_at,
                    },
                ))
            })
            .map_err(|e| RedberryError::Cache(e.to_string()))?;

        let mut results = Vec::new();
        for msg in messages_iter {
            results.push(msg.map_err(|e| RedberryError::Cache(e.to_string()))?);
        }

        Ok(results)
    }

    /// Evict sessions older than `ttl_hours`.
    pub fn evict_stale(&self, ttl_hours: u32) -> Result<usize, RedberryError> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (ttl_hours as i64 * 3600);

        let count = self
            .conn
            .execute(
                "DELETE FROM sessions WHERE updated_at < ?1",
                params![cutoff],
            )
            .map_err(|e| RedberryError::Cache(e.to_string()))?;

        // SQLite doesn't auto-cascade deletes in the same way without pragma foreign_keys=ON,
        // but rusqlite has it on by default for new versions. Let's ensure manual cleanup just in case.
        self.conn
            .execute(
                "DELETE FROM messages WHERE session_id NOT IN (SELECT id FROM sessions)",
                [],
            )
            .map_err(|e| RedberryError::Cache(e.to_string()))?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_cache() -> ContextCache {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                consecutive_bad INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE messages (
                session_id TEXT NOT NULL,
                idx INTEGER NOT NULL,
                text TEXT NOT NULL,
                embedding BLOB NOT NULL,
                snark_response TEXT,
                metrics_vagueness REAL NOT NULL DEFAULT 0.0,
                metrics_syntax REAL NOT NULL DEFAULT 0.0,
                metrics_drift REAL NOT NULL DEFAULT 0.0,
                PRIMARY KEY (session_id, idx)
            )",
            [],
        )
        .unwrap();
        ContextCache { conn }
    }

    #[test]
    fn test_store_and_retrieve_context() {
        let mut cache = in_memory_cache();
        let session_id = "test-session";

        let messages = vec![
            ContextMessage {
                text: "Hello".to_string(),
                embedding: vec![0.1, 0.2, 0.3],
                snark_response: None,
                metrics_vagueness: 0.0,
                metrics_syntax: 0.0,
                metrics_drift: 0.0,
                created_at: None,
            },
            ContextMessage {
                text: "World".to_string(),
                embedding: vec![0.4, 0.5, 0.6],
                snark_response: Some("Hello world to you!".to_string()),
                metrics_vagueness: 0.4,
                metrics_syntax: 0.1,
                metrics_drift: 0.3,
                created_at: None,
            },
        ];

        let ctx = SessionContext {
            session_id: session_id.to_string(),
            messages,
            consecutive_bad: 0,
        };

        cache.store_context(&ctx).unwrap();

        let retrieved = cache.get_context(session_id).unwrap().unwrap();
        assert_eq!(retrieved.session_id, session_id);
        assert_eq!(retrieved.messages.len(), 2);
        assert_eq!(retrieved.messages[0].text, "Hello");
        assert_eq!(retrieved.messages[0].embedding, vec![0.1, 0.2, 0.3]);
        assert_eq!(retrieved.messages[1].text, "World");
    }

    #[test]
    fn test_append_messages() {
        let mut cache = in_memory_cache();
        let session_id = "append-session";

        cache
            .append_messages(
                session_id,
                &[ContextMessage {
                    text: "Msg 1".to_string(),
                    embedding: vec![0.1],
                    snark_response: None,
                    metrics_vagueness: 0.0,
                    metrics_syntax: 0.0,
                    metrics_drift: 0.0,
                    created_at: None,
                }],
                0,
            )
            .unwrap();

        cache
            .append_messages(
                session_id,
                &[ContextMessage {
                    text: "Msg 2".to_string(),
                    embedding: vec![0.2],
                    snark_response: None,
                    metrics_vagueness: 0.0,
                    metrics_syntax: 0.0,
                    metrics_drift: 0.0,
                    created_at: None,
                }],
                1,
            )
            .unwrap();

        let retrieved = cache.get_context(session_id).unwrap().unwrap();
        assert_eq!(retrieved.messages.len(), 2);
        assert_eq!(retrieved.messages[1].text, "Msg 2");
    }
}
