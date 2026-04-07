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

    pub fn upsert_file_state(
        &self,
        path: &str,
        hash: &str,
        size: i64,
        ts_ns: i64,
    ) -> Result<()> {
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
pub struct EventRow {
    pub id: i64,
    pub ts_ns: i64,
    pub path: String,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
    pub size_before: Option<i64>,
    pub size_after: Option<i64>,
    pub attribution: String,
    pub session_id: Option<String>,
}
