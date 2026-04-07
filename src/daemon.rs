use anyhow::Result;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::hook::{read_active_session, ActiveSession};
use crate::store::{NewEvent, Store};

/// Files larger than this are skipped entirely. Mostly to avoid snapshotting
/// build artifacts or media assets. Configurable in v0.2.
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Walk the project tree and snapshot every file into the CAS, recording each
/// as an `initial-scan` event. Called once from `agent-undo init` so that
/// subsequent FS events have a coherent "before" state to reference.
pub fn initial_scan(store: &Store) -> Result<usize> {
    let root = store.paths.root.clone();
    let ts_ns = now_ns();
    let mut count = 0;

    let walker = ignore::WalkBuilder::new(&root)
        .add_custom_ignore_filename(".agent-undoignore")
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name != ".agent-undo" && name != ".git"
        })
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > MAX_FILE_SIZE {
            continue;
        }
        let (hash, bytes) = match Store::hash_file(path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let size = bytes.len() as i64;
        let rel = relative_path(&root, path);

        store.write_blob(&bytes)?;
        store.record_event(&NewEvent {
            ts_ns,
            path: rel.clone(),
            before_hash: None,
            after_hash: Some(hash.clone()),
            size_before: None,
            size_after: Some(size),
            attribution: "initial-scan".into(),
            confidence: "high".into(),
            session_id: None,
            pid: None,
            process_name: None,
            tool_name: None,
        })?;
        store.upsert_file_state(&rel, &hash, size, ts_ns)?;
        count += 1;
    }
    Ok(count)
}

/// Run the FS watcher loop. Blocks the current thread. Each detected change
/// is hashed, written to the CAS if new, and appended to the timeline.
///
/// v0 is foreground-only and synchronous. Real daemonization (unix socket,
/// launchd/systemd integration) is a v0.2 task.
pub fn serve(store: Store) -> Result<()> {
    let root = store.paths.root.clone();
    tracing::info!("agent-undo watching {}", root.display());

    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    let ignorer = build_ignorer(&root);

    while let Ok(res) = rx.recv() {
        match res {
            Ok(event) => {
                if let Err(e) = handle_event(&store, &ignorer, event) {
                    tracing::warn!("handle_event error: {:?}", e);
                }
            }
            Err(e) => tracing::warn!("watch error: {:?}", e),
        }
    }
    Ok(())
}

fn handle_event(store: &Store, ignorer: &Gitignore, event: notify::Event) -> Result<()> {
    let relevant = matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    );
    if !relevant {
        return Ok(());
    }

    for path in event.paths {
        if should_skip(ignorer, &path) {
            continue;
        }
        if let Err(e) = process_path(store, &path) {
            tracing::warn!("process_path {}: {:?}", path.display(), e);
        }
    }
    Ok(())
}

fn process_path(store: &Store, path: &Path) -> Result<()> {
    let rel = relative_path(&store.paths.root, path);
    let ts_ns = now_ns();

    // If the path no longer exists, this is a deletion.
    if !path.exists() {
        if let Some((before_hash, size_before)) = store.get_file_state(&rel)? {
            store.record_event(&NewEvent {
                ts_ns,
                path: rel.clone(),
                before_hash: Some(before_hash),
                after_hash: None,
                size_before: Some(size_before),
                size_after: None,
                attribution: "unknown".into(),
                confidence: "none".into(),
                session_id: None,
                pid: None,
                process_name: None,
                tool_name: None,
            })?;
            store.delete_file_state(&rel)?;
            tracing::info!("deleted: {}", rel);
        }
        return Ok(());
    }

    // Directories surface as events for their children; skip the dir event itself.
    let meta = std::fs::metadata(path)?;
    if meta.is_dir() {
        return Ok(());
    }
    if meta.len() > MAX_FILE_SIZE {
        return Ok(());
    }

    let (hash, bytes) = Store::hash_file(path)?;
    let size = bytes.len() as i64;

    let prior = store.get_file_state(&rel)?;
    if let Some((ref prev_hash, _)) = prior {
        if prev_hash == &hash {
            return Ok(()); // content unchanged; common case during editor saves
        }
    }

    let attribution = resolve_attribution(store);

    store.write_blob(&bytes)?;
    store.record_event(&NewEvent {
        ts_ns,
        path: rel.clone(),
        before_hash: prior.as_ref().map(|(h, _)| h.clone()),
        after_hash: Some(hash.clone()),
        size_before: prior.as_ref().map(|(_, s)| *s),
        size_after: Some(size),
        attribution: attribution.agent,
        confidence: attribution.confidence,
        session_id: attribution.session_id,
        pid: None,
        process_name: None,
        tool_name: attribution.tool_name,
    })?;
    store.upsert_file_state(&rel, &hash, size, ts_ns)?;
    tracing::info!("snapshot: {} ({})", rel, &hash[..12]);
    Ok(())
}

#[derive(Debug, Clone)]
struct Attribution {
    agent: String,
    confidence: String,
    session_id: Option<String>,
    tool_name: Option<String>,
}

/// Resolve the "who wrote this file" answer by checking (in order):
///   Layer 2: `.agent-undo/active-session.json` (Claude Code hook)
///   Layer 0: "unknown"
///
/// Layer 1 (process scanning heuristic) and Layer 3 (eBPF) are v0.3.
fn resolve_attribution(store: &Store) -> Attribution {
    if let Ok(Some(active)) = read_active_session(&store.paths) {
        return from_active(&active);
    }
    Attribution {
        agent: "unknown".into(),
        confidence: "none".into(),
        session_id: None,
        tool_name: None,
    }
}

fn from_active(active: &ActiveSession) -> Attribution {
    Attribution {
        agent: active.agent.clone(),
        confidence: "high".into(),
        session_id: Some(active.session_id.clone()),
        tool_name: active.tool_name.clone(),
    }
}

fn should_skip(ignorer: &Gitignore, path: &Path) -> bool {
    // Always skip anything inside .agent-undo/ itself — otherwise we'd snapshot
    // our own blobs in an infinite loop.
    if path
        .components()
        .any(|c| c.as_os_str() == ".agent-undo" || c.as_os_str() == ".git")
    {
        return true;
    }
    let is_dir = path.is_dir();
    ignorer.matched(path, is_dir).is_ignore()
}

fn build_ignorer(root: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(root);
    // User gitignore wins first.
    let _ = builder.add(root.join(".gitignore"));
    let _ = builder.add(root.join(".agent-undoignore"));
    // Hard-coded safe defaults.
    for pat in [
        ".agent-undo/",
        ".git/",
        "target/",
        "node_modules/",
        "dist/",
        "build/",
        ".DS_Store",
    ] {
        let _ = builder.add_line(None, pat);
    }
    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

