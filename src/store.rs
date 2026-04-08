use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::paths::ProjectPaths;

/// The agent-undo storage layer.
///
/// Wraps the content-addressable blob store (`.agent-undo/objects/`) and the
/// SQLite timeline database (`.agent-undo/timeline.db`). Everything the daemon
/// and CLI read or write to disk goes through this.
pub struct Store {
    pub paths: ProjectPaths,
    pub conn: Connection,
}

impl Store {
    /// Open an existing store. Errors if the DB file is missing.
    pub fn open(paths: ProjectPaths) -> Result<Self> {
        let conn = Connection::open(&paths.db_path)
            .with_context(|| format!("opening {}", paths.db_path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { paths, conn })
    }

    /// Create a fresh store: `.agent-undo/` directory, objects dir, and DB schema.
    pub fn init(paths: ProjectPaths) -> Result<Self> {
        fs::create_dir_all(&paths.objects_dir)
            .with_context(|| format!("creating {}", paths.objects_dir.display()))?;
        let store = Self::open(paths)?;
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                ts_ns        INTEGER NOT NULL,
                path         TEXT NOT NULL,
                before_hash  TEXT,
                after_hash   TEXT,
                size_before  INTEGER,
                size_after   INTEGER,
                attribution  TEXT NOT NULL DEFAULT 'unknown',
                confidence   TEXT NOT NULL DEFAULT 'none',
                session_id   TEXT,
                pid          INTEGER,
                process_name TEXT,
                tool_name    TEXT,
                metadata     TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_events_path_ts  ON events(path, ts_ns DESC);
            CREATE INDEX IF NOT EXISTS idx_events_session  ON events(session_id);
            CREATE INDEX IF NOT EXISTS idx_events_agent_ts ON events(attribution, ts_ns DESC);

            CREATE TABLE IF NOT EXISTS sessions (
                id            TEXT PRIMARY KEY,
                agent         TEXT NOT NULL,
                started_at_ns INTEGER NOT NULL,
                ended_at_ns   INTEGER,
                prompt        TEXT,
                model         TEXT,
                metadata      TEXT
            );

            CREATE TABLE IF NOT EXISTS pins (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id      INTEGER NOT NULL,
                label         TEXT NOT NULL,
                created_at_ns INTEGER NOT NULL
            );

            -- file_state tracks the latest known hash for each path so we can
            -- cheaply decide whether an FS event reflects a real content change.
            CREATE TABLE IF NOT EXISTS file_state (
                path         TEXT PRIMARY KEY,
                latest_hash  TEXT NOT NULL,
                latest_size  INTEGER NOT NULL,
                latest_ts_ns INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    // --- blob store -------------------------------------------------------

    #[allow(dead_code)] // used by GC + restore verification in v0.3
    pub fn has_blob(&self, hash: &str) -> bool {
        self.paths.object_path(hash).exists()
    }

    /// Write bytes into the CAS, keyed by their BLAKE3 hash. No-op if the blob
    /// already exists. Writes are atomic via temp-file-then-rename.
    pub fn write_blob(&self, bytes: &[u8]) -> Result<String> {
        let hash = blake3::hash(bytes).to_hex().to_string();
        let path = self.paths.object_path(&hash);
        if path.exists() {
            return Ok(hash);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &path)?;
        Ok(hash)
    }

    pub fn read_blob(&self, hash: &str) -> Result<Vec<u8>> {
        Ok(fs::read(self.paths.object_path(hash))?)
    }

    /// Hash a file from disk and return its bytes so we can deduplicate-check
    /// before writing a blob.
    pub fn hash_file(path: &Path) -> Result<(String, Vec<u8>)> {
        let bytes = fs::read(path)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        Ok((hash, bytes))
    }

    // --- timeline ---------------------------------------------------------

    pub fn get_file_state(&self, path: &str) -> Result<Option<(String, i64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT latest_hash, latest_size FROM file_state WHERE path = ?1")?;
        let result = stmt.query_row(params![path], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        });
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn upsert_file_state(&self, path: &str, hash: &str, size: i64, ts_ns: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO file_state (path, latest_hash, latest_size, latest_ts_ns)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET
                latest_hash = excluded.latest_hash,
                latest_size = excluded.latest_size,
                latest_ts_ns = excluded.latest_ts_ns",
            params![path, hash, size, ts_ns],
        )?;
        Ok(())
    }

    pub fn delete_file_state(&self, path: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM file_state WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn record_event(&self, event: &NewEvent) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO events (
                ts_ns, path, before_hash, after_hash, size_before, size_after,
                attribution, confidence, session_id, pid, process_name, tool_name
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                event.ts_ns,
                event.path,
                event.before_hash,
                event.after_hash,
                event.size_before,
                event.size_after,
                event.attribution,
                event.confidence,
                event.session_id,
                event.pid,
                event.process_name,
                event.tool_name,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_event(&self, id: i64) -> Result<Option<EventRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ts_ns, path, before_hash, after_hash, size_before, size_after, attribution, session_id
             FROM events WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(EventRow {
                id: row.get(0)?,
                ts_ns: row.get(1)?,
                path: row.get(2)?,
                before_hash: row.get(3)?,
                after_hash: row.get(4)?,
                size_before: row.get(5)?,
                size_after: row.get(6)?,
                attribution: row.get(7)?,
                session_id: row.get(8)?,
            })
        });
        match result {
            Ok(ev) => Ok(Some(ev)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Most recent event for a specific file path, excluding restore-driven
    /// events (so `restore --file X` walks past its own prior restores).
    pub fn latest_user_event_for_file(&self, path: &str) -> Result<Option<EventRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ts_ns, path, before_hash, after_hash, size_before, size_after, attribution, session_id
             FROM events
             WHERE path = ?1
               AND attribution NOT IN ('agent-undo-restore', 'pre-restore', 'initial-scan')
             ORDER BY id DESC LIMIT 1",
        )?;
        let result = stmt.query_row(params![path], |row| {
            Ok(EventRow {
                id: row.get(0)?,
                ts_ns: row.get(1)?,
                path: row.get(2)?,
                before_hash: row.get(3)?,
                after_hash: row.get(4)?,
                size_before: row.get(5)?,
                size_after: row.get(6)?,
                attribution: row.get(7)?,
                session_id: row.get(8)?,
            })
        });
        match result {
            Ok(ev) => Ok(Some(ev)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn events_for_session(&self, session_id: &str) -> Result<Vec<EventRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ts_ns, path, before_hash, after_hash, size_before, size_after, attribution, session_id
             FROM events
             WHERE session_id = ?1
               AND attribution NOT IN ('agent-undo-restore', 'pre-restore')
             ORDER BY ts_ns ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(EventRow {
                id: row.get(0)?,
                ts_ns: row.get(1)?,
                path: row.get(2)?,
                before_hash: row.get(3)?,
                after_hash: row.get(4)?,
                size_before: row.get(5)?,
                size_after: row.get(6)?,
                attribution: row.get(7)?,
                session_id: row.get(8)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Filtered timeline query. Pass `None` for any filter to disable it.
    pub fn filtered_events(
        &self,
        agent: Option<&str>,
        path_substring: Option<&str>,
        since_ns: Option<i64>,
        limit: usize,
    ) -> Result<Vec<EventRow>> {
        let mut sql = String::from(
            "SELECT id, ts_ns, path, before_hash, after_hash, size_before, size_after, attribution, session_id
             FROM events WHERE 1=1",
        );
        let mut bound: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(a) = agent {
            sql.push_str(" AND attribution = ?");
            bound.push(rusqlite::types::Value::Text(a.to_string()));
        }
        if let Some(p) = path_substring {
            sql.push_str(" AND path LIKE ?");
            bound.push(rusqlite::types::Value::Text(format!("%{p}%")));
        }
        if let Some(ts) = since_ns {
            sql.push_str(" AND ts_ns >= ?");
            bound.push(rusqlite::types::Value::Integer(ts));
        }
        sql.push_str(" ORDER BY id DESC LIMIT ?");
        bound.push(rusqlite::types::Value::Integer(limit as i64));

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            bound.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(EventRow {
                id: row.get(0)?,
                ts_ns: row.get(1)?,
                path: row.get(2)?,
                before_hash: row.get(3)?,
                after_hash: row.get(4)?,
                size_before: row.get(5)?,
                size_after: row.get(6)?,
                attribution: row.get(7)?,
                session_id: row.get(8)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn recent_events(&self, limit: usize) -> Result<Vec<EventRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ts_ns, path, before_hash, after_hash, size_before, size_after, attribution, session_id
             FROM events ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(EventRow {
                id: row.get(0)?,
                ts_ns: row.get(1)?,
                path: row.get(2)?,
                before_hash: row.get(3)?,
                after_hash: row.get(4)?,
                size_before: row.get(5)?,
                size_after: row.get(6)?,
                attribution: row.get(7)?,
                session_id: row.get(8)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    // --- sessions ---------------------------------------------------------

    /// Insert or update a session. Idempotent on session id.
    pub fn upsert_session(&self, s: &SessionRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sessions (id, agent, started_at_ns, ended_at_ns, prompt, model, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                agent         = excluded.agent,
                ended_at_ns   = COALESCE(excluded.ended_at_ns, sessions.ended_at_ns),
                prompt        = COALESCE(excluded.prompt, sessions.prompt),
                model         = COALESCE(excluded.model, sessions.model),
                metadata      = COALESCE(excluded.metadata, sessions.metadata)",
            params![
                s.id,
                s.agent,
                s.started_at_ns,
                s.ended_at_ns,
                s.prompt,
                s.model,
                s.metadata,
            ],
        )?;
        Ok(())
    }

    pub fn mark_session_ended(&self, session_id: &str, ts_ns: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET ended_at_ns = ?1 WHERE id = ?2 AND ended_at_ns IS NULL",
            params![ts_ns, session_id],
        )?;
        Ok(())
    }

    pub fn list_sessions(&self, limit: usize) -> Result<Vec<SessionRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent, started_at_ns, ended_at_ns, prompt, model, metadata
             FROM sessions ORDER BY started_at_ns DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                agent: row.get(1)?,
                started_at_ns: row.get(2)?,
                ended_at_ns: row.get(3)?,
                prompt: row.get(4)?,
                model: row.get(5)?,
                metadata: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    // --- pins -------------------------------------------------------------

    pub fn create_pin(&self, label: &str) -> Result<i64> {
        // Pin against the *latest* event id, or 0 if there are none yet.
        let event_id: i64 = self
            .conn
            .query_row("SELECT COALESCE(MAX(id), 0) FROM events", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);
        let ts_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);
        self.conn.execute(
            "INSERT INTO pins (event_id, label, created_at_ns) VALUES (?1, ?2, ?3)",
            params![event_id, label, ts_ns],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn find_pin(&self, label: &str) -> Result<Option<PinRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_id, label, created_at_ns FROM pins
             WHERE label = ?1 ORDER BY created_at_ns DESC LIMIT 1",
        )?;
        let result = stmt.query_row(params![label], |row| {
            Ok(PinRow {
                id: row.get(0)?,
                event_id: row.get(1)?,
                label: row.get(2)?,
                created_at_ns: row.get(3)?,
            })
        });
        match result {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Snapshot of every file's state as of just before `event_id`. Used by
    /// `restore --pin`: we want to restore the WHOLE PROJECT to its state at
    /// the moment the pin was created.
    pub fn file_state_at_event(&self, event_id: i64) -> Result<Vec<(String, Option<String>)>> {
        // For each path, find the most recent event with id <= event_id
        // and take its after_hash (or None if the file was deleted at that point).
        let mut stmt = self.conn.prepare(
            "SELECT path, after_hash FROM events e1
             WHERE id = (
                SELECT MAX(id) FROM events e2
                WHERE e2.path = e1.path AND e2.id <= ?1
             )",
        )?;
        let rows = stmt.query_map(params![event_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    #[allow(dead_code)] // surfaced by `agent-undo pin --list` in v0.3
    pub fn list_pins(&self) -> Result<Vec<PinRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_id, label, created_at_ns FROM pins ORDER BY created_at_ns DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PinRow {
                id: row.get(0)?,
                event_id: row.get(1)?,
                label: row.get(2)?,
                created_at_ns: row.get(3)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Garbage-collect events older than `older_than_ns` AND not referenced
    /// by any pin. Returns (events_deleted, blobs_deleted).
    pub fn gc(&self, older_than_ns: i64) -> Result<(usize, usize)> {
        let pin_event_ids: Vec<i64> = self
            .conn
            .prepare("SELECT DISTINCT event_id FROM pins")?
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Delete events older than the cutoff that are not pinned.
        let cutoff_ts: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0)
            - older_than_ns;

        // Don't drop the most recent event for each path — that's the live
        // state and the dedup baseline.
        let mut stmt = self.conn.prepare(
            "SELECT id FROM events
             WHERE ts_ns < ?1
               AND id NOT IN (SELECT MAX(id) FROM events GROUP BY path)
               AND id NOT IN (SELECT event_id FROM pins)",
        )?;
        let to_delete: Vec<i64> = stmt
            .query_map(params![cutoff_ts], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);

        let mut events_deleted = 0usize;
        for id in &to_delete {
            self.conn
                .execute("DELETE FROM events WHERE id = ?1", params![id])?;
            events_deleted += 1;
        }
        let _ = pin_event_ids; // currently informational; reserved for v0.3 strict mode

        let blobs_deleted = self.sweep_orphan_blobs()?;

        Ok((events_deleted, blobs_deleted))
    }

    pub fn sweep_orphan_blobs(&self) -> Result<usize> {
        let referenced: std::collections::HashSet<String> = self
            .conn
            .prepare(
                "SELECT before_hash FROM events WHERE before_hash IS NOT NULL
                 UNION SELECT after_hash FROM events WHERE after_hash IS NOT NULL
                 UNION SELECT latest_hash FROM file_state",
            )?
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        let mut blobs_deleted = 0usize;
        if let Ok(entries) = std::fs::read_dir(&self.paths.objects_dir) {
            for shard in entries.flatten() {
                let shard_path = shard.path();
                if !shard_path.is_dir() {
                    continue;
                }
                let shard_name = shard_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if let Ok(blobs) = std::fs::read_dir(&shard_path) {
                    for blob in blobs.flatten() {
                        let blob_path = blob.path();
                        let rest = blob_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();
                        let hash = format!("{shard_name}{rest}");
                        if !referenced.contains(&hash) && std::fs::remove_file(&blob_path).is_ok() {
                            blobs_deleted += 1;
                        }
                    }
                }
            }
        }

        Ok(blobs_deleted)
    }

    pub fn event_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?)
    }
}

/// An event to be inserted. Most fields are optional because v0 attribution is
/// best-effort; later layers (hook, MCP, exec wrapper) fill in more.
#[derive(Debug, Clone)]
pub struct NewEvent {
    pub ts_ns: i64,
    pub path: String,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
    pub size_before: Option<i64>,
    pub size_after: Option<i64>,
    pub attribution: String,
    pub confidence: String,
    pub session_id: Option<String>,
    pub pid: Option<i64>,
    pub process_name: Option<String>,
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // surfaced by `agent-undo pin` listing in v0.3
pub struct PinRow {
    pub id: i64,
    pub event_id: i64,
    pub label: String,
    pub created_at_ns: i64,
}

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub agent: String,
    pub started_at_ns: i64,
    pub ended_at_ns: Option<i64>,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EventRow {
    pub id: i64,
    pub ts_ns: i64,
    pub path: String,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
    #[allow(dead_code)] // surfaced by TUI / blame in v0.3
    pub size_before: Option<i64>,
    #[allow(dead_code)] // surfaced by TUI / blame in v0.3
    pub size_after: Option<i64>,
    pub attribution: String,
    #[allow(dead_code)] // surfaced by `diff --session` in v0.3
    pub session_id: Option<String>,
}
