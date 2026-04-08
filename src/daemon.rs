use anyhow::Result;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{EventKind, RecursiveMode, Watcher};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Process, ProcessRefreshKind, RefreshKind, System};

use crate::config::AppConfig;
use crate::hook::{read_active_session, ActiveSession};
use crate::ipc;
use crate::store::{NewEvent, Store};

const COALESCE_WINDOW_MS: u64 = 100;

/// Walk the project tree and snapshot every file into the CAS, recording each
/// as an `initial-scan` event. Called once from `agent-undo init` so that
/// subsequent FS events have a coherent "before" state to reference.
pub fn initial_scan(store: &Store) -> Result<usize> {
    let config = AppConfig::load(&store.paths)?;
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
        if meta.len() > config.max_file_size_bytes() {
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
    let config = AppConfig::load(&store.paths)?;
    let root = store.paths.root.clone();
    tracing::info!("agent-undo watching {}", root.display());
    let socket_guard = ipc::spawn_server(store.paths.clone())?;

    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    let ignorer = build_ignorer(&root, &config);

    while let Ok(first) = rx.recv() {
        let mut batch = vec![first];
        loop {
            match rx.recv_timeout(Duration::from_millis(COALESCE_WINDOW_MS)) {
                Ok(res) => batch.push(res),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        if let Err(e) = handle_batch(&store, &ignorer, &config, batch) {
            tracing::warn!("handle_batch error: {:?}", e);
        }
    }
    drop(socket_guard);
    Ok(())
}

fn handle_batch(
    store: &Store,
    ignorer: &Gitignore,
    config: &AppConfig,
    batch: Vec<notify::Result<notify::Event>>,
) -> Result<()> {
    let mut candidates = BTreeSet::new();

    for res in batch {
        let event = match res {
            Ok(event) => event,
            Err(e) => {
                tracing::warn!("watch error: {:?}", e);
                continue;
            }
        };
        let relevant = matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        );
        if !relevant {
            continue;
        }

        for path in event.paths {
            if should_skip(ignorer, &path) || should_skip_transient(store, &path)? {
                continue;
            }
            let _ = candidates.insert(path);
        }
    }

    for path in candidates {
        if let Err(e) = process_path(store, &path, config) {
            tracing::warn!("process_path {}: {:?}", path.display(), e);
        }
    }

    Ok(())
}

fn process_path(store: &Store, path: &Path, config: &AppConfig) -> Result<()> {
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
    if meta.len() > config.max_file_size_bytes() {
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
        pid: attribution.pid,
        process_name: attribution.process_name,
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
    pid: Option<i64>,
    process_name: Option<String>,
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
    if let Some(attribution) = heuristic_attribution(&store.paths.root) {
        return attribution;
    }
    Attribution {
        agent: "unknown".into(),
        confidence: "none".into(),
        session_id: None,
        tool_name: None,
        pid: None,
        process_name: None,
    }
}

fn from_active(active: &ActiveSession) -> Attribution {
    Attribution {
        agent: active.agent.clone(),
        confidence: "high".into(),
        session_id: Some(active.session_id.clone()),
        tool_name: active.tool_name.clone(),
        pid: None,
        process_name: None,
    }
}

fn heuristic_attribution(root: &Path) -> Option<Attribution> {
    let system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    let self_pid = std::process::id();
    let mut best: Option<(u8, Attribution)> = None;

    for process in system.processes().values() {
        let pid = process.pid().as_u32();
        if pid == self_pid {
            continue;
        }
        let Some(cwd) = process.cwd() else {
            continue;
        };
        if !cwd.starts_with(root) {
            continue;
        }
        let Some((agent, score)) = fingerprint_process(process) else {
            continue;
        };
        let attribution = Attribution {
            agent: agent.into(),
            confidence: if score >= 30 { "medium" } else { "low" }.into(),
            session_id: None,
            tool_name: None,
            pid: Some(pid as i64),
            process_name: Some(process.name().to_string()),
        };
        if best
            .as_ref()
            .map(|(best_score, _)| score > *best_score)
            .unwrap_or(true)
        {
            best = Some((score, attribution));
        }
    }

    best.map(|(_, attribution)| attribution)
}

fn fingerprint_process(process: &Process) -> Option<(&'static str, u8)> {
    let name = process.name().to_ascii_lowercase();
    if name == "au" || name.contains("agent-undo") {
        return None;
    }

    let exe = process
        .exe()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let cmd = process.cmd().join(" ").to_ascii_lowercase();
    let haystack = format!("{name} {exe} {cmd}");

    if haystack.contains("cursor") {
        return Some(("cursor", 40));
    }
    if haystack.contains("claude") {
        return Some(("claude-code", 40));
    }
    if haystack.contains("aider") {
        return Some(("aider", 35));
    }
    if haystack.contains("codex") {
        return Some(("codex", 35));
    }
    if haystack.contains("cline") || haystack.contains("roo") {
        return Some(("cline", 30));
    }
    if haystack.contains("continue") {
        return Some(("continue", 30));
    }

    None
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

fn should_skip_transient(store: &Store, path: &Path) -> Result<bool> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(false);
    };
    let looks_transient = name.ends_with(".tmp")
        || name.ends_with(".temp")
        || name.ends_with(".swp")
        || name.ends_with(".swo")
        || name.ends_with('~')
        || name.ends_with("___jb_tmp___")
        || name.ends_with("___jb_old___")
        || name.starts_with(".#");
    if !looks_transient {
        return Ok(false);
    }

    let rel = relative_path(&store.paths.root, path);
    Ok(store.get_file_state(&rel)?.is_none())
}

fn build_ignorer(root: &Path, config: &AppConfig) -> Gitignore {
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
    for pat in &config.watch.ignore_patterns {
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

#[cfg(test)]
mod tests {
    use super::fingerprint_process;
    use sysinfo::{ProcessRefreshKind, RefreshKind, System};

    #[test]
    fn fingerprint_tolerates_real_process_handles() {
        let system = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
        );
        let current = system
            .processes()
            .values()
            .find(|process| process.pid().as_u32() == std::process::id())
            .expect("current process present");

        let _ = fingerprint_process(current);
    }
}
