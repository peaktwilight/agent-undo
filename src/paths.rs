use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// Well-known paths for an agent-undo project root.
///
/// `data_dir` is `<root>/.agent-undo/`.
/// `objects_dir` is the content-addressable blob store.
/// `db_path` is the SQLite timeline database.
#[derive(Debug, Clone)]
pub struct ProjectPaths {
    pub root: PathBuf,
    pub data_dir: PathBuf,
    pub objects_dir: PathBuf,
    pub db_path: PathBuf,
    pub config_path: PathBuf,
    pub socket_path: PathBuf,
}

impl ProjectPaths {
    /// Walk upward from the current directory looking for an existing
    /// `.agent-undo/` directory. Errors if none is found.
    pub fn discover() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let mut cur: &Path = &cwd;
        loop {
            if cur.join(".agent-undo").is_dir() {
                return Ok(Self::for_root(cur.to_path_buf()));
            }
            match cur.parent() {
                Some(p) => cur = p,
                None => bail!(
                    "no .agent-undo/ found from {}. Run `agent-undo init` first.",
                    cwd.display()
                ),
            }
        }
    }

    /// Treat the current directory as the project root (for `init`).
    pub fn cwd_as_root() -> Result<Self> {
        Ok(Self::for_root(std::env::current_dir()?))
    }

    pub fn for_root(root: PathBuf) -> Self {
        let data_dir = root.join(".agent-undo");
        let objects_dir = data_dir.join("objects");
        let db_path = data_dir.join("timeline.db");
        let config_path = data_dir.join("config.toml");
        let socket_path = data_dir.join("daemon.sock");
        Self {
            root,
            data_dir,
            objects_dir,
            db_path,
            config_path,
            socket_path,
        }
    }

    /// Path for a given blob hash, split into a 2-char subdir to avoid
    /// directories with hundreds of thousands of entries.
    pub fn object_path(&self, hash: &str) -> PathBuf {
        self.objects_dir.join(&hash[..2]).join(&hash[2..])
    }
}
