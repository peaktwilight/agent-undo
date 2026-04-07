// restore.rs — the actual undo machinery.
//
// Every restore follows the same two-step dance:
//
//   1. Snapshot the current state of the target file as a `pre-restore` event.
//      This is the safety rule from PHILOSOPHY.md: we never destroy data to
//      recover data. Undo-the-undo is always one command away.
//
//   2. Write (or delete) the target state, and record it as an
//      `agent-undo-restore` event. The file_state table is updated so the
//      daemon's dedup check sees the new state as current and doesn't snapshot
//      the write-back as a spurious user edit.
//
// The oops/session variants are just bulk applications of `restore_file_to`.

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::store::{EventRow, NewEvent, Store};

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

/// Restore a single file to the content represented by `target_hash`.
/// `target_hash = None` means "the file did not exist" — delete it.
///
/// Always records a pre-restore snapshot of the current file state first.
pub fn restore_file_to(store: &Store, rel_path: &str, target_hash: Option<&str>) -> Result<()> {
    let abs = store.paths.root.join(rel_path);
    let ts_ns = now_ns();

    // Capture current state so the restore is itself reversible.
    let current: Option<(String, i64, Vec<u8>)> = if abs.exists() && abs.is_file() {
        let bytes = fs::read(&abs).with_context(|| format!("reading current {}", abs.display()))?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let size = bytes.len() as i64;
        store.write_blob(&bytes)?;
        Some((hash, size, bytes))
    } else {
        None
    };

    // Record the pre-restore snapshot. Before and after are intentionally the
    // same hash — this event is a marker, not a content change.
    store.record_event(&NewEvent {
        ts_ns,
        path: rel_path.into(),
        before_hash: current.as_ref().map(|(h, _, _)| h.clone()),
        after_hash: current.as_ref().map(|(h, _, _)| h.clone()),
        size_before: current.as_ref().map(|(_, s, _)| *s),
        size_after: current.as_ref().map(|(_, s, _)| *s),
        attribution: "pre-restore".into(),
        confidence: "high".into(),
        session_id: None,
        pid: None,
        process_name: None,
        tool_name: None,
    })?;

    // Apply the target state.
    match target_hash {
        Some(h) => {
            let bytes = store
                .read_blob(h)
                .with_context(|| format!("reading blob {h}"))?;
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent)?;
            }
            // Atomic write via temp + rename to match the store's invariant.
            let tmp = abs.with_extension(format!("agent-undo-restore.{}", &h[..8.min(h.len())]));
            fs::write(&tmp, &bytes)?;
            fs::rename(&tmp, &abs)?;

            let size = bytes.len() as i64;
            let after_ts = now_ns();
            store.record_event(&NewEvent {
                ts_ns: after_ts,
                path: rel_path.into(),
                before_hash: current.as_ref().map(|(h, _, _)| h.clone()),
                after_hash: Some(h.into()),
                size_before: current.as_ref().map(|(_, s, _)| *s),
                size_after: Some(size),
                attribution: "agent-undo-restore".into(),
                confidence: "high".into(),
                session_id: None,
                pid: None,
                process_name: None,
                tool_name: None,
            })?;
            store.upsert_file_state(rel_path, h, size, after_ts)?;
        }
        None => {
            // Target says "file didn't exist" — delete it if present.
            if abs.exists() {
                fs::remove_file(&abs).with_context(|| format!("removing {}", abs.display()))?;
            }
            let after_ts = now_ns();
            store.record_event(&NewEvent {
                ts_ns: after_ts,
                path: rel_path.into(),
                before_hash: current.as_ref().map(|(h, _, _)| h.clone()),
                after_hash: None,
                size_before: current.as_ref().map(|(_, s, _)| *s),
                size_after: None,
                attribution: "agent-undo-restore".into(),
                confidence: "high".into(),
                session_id: None,
                pid: None,
                process_name: None,
                tool_name: None,
            })?;
            store.delete_file_state(rel_path)?;
        }
    }

    Ok(())
}

/// Restore a file to its state BEFORE a given event happened. This is the
/// semantics of `agent-undo restore <event-id>` — "undo this event."
pub fn restore_to_event(store: &Store, event: &EventRow) -> Result<()> {
    restore_file_to(store, &event.path, event.before_hash.as_deref())
}

/// `restore --file F`: walk back to the state before the most recent
/// user-originated change on F.
pub fn restore_latest_change_to_file(store: &Store, rel_path: &str) -> Result<EventRow> {
    let ev = store
        .latest_user_event_for_file(rel_path)?
        .ok_or_else(|| anyhow::anyhow!("no undoable events for {rel_path}"))?;
    restore_to_event(store, &ev)?;
    Ok(ev)
}

/// Plan and execute `oops`: walk back from the most recent user-originated
/// event, collecting everything within `window_ns`, and restore each affected
/// file to the oldest pre-state in that batch.
///
/// Returns the list of (rel_path, oldest_event_in_batch) actually applied.
pub fn oops(store: &Store, window_ns: i64) -> Result<Vec<(String, EventRow)>> {
    let batch = oops_plan(store, window_ns)?;
    for (path, ev) in &batch {
        restore_file_to(store, path, ev.before_hash.as_deref())?;
    }
    Ok(batch)
}

/// The read-only "what would oops do?" computation.
pub fn oops_plan(store: &Store, window_ns: i64) -> Result<Vec<(String, EventRow)>> {
    let events = store.recent_events(500)?;
    let undoable: Vec<&EventRow> = events
        .iter()
        .filter(|e| !is_restore_bookkeeping(&e.attribution))
        .collect();

    if undoable.is_empty() {
        return Ok(vec![]);
    }

    let most_recent_ts = undoable[0].ts_ns;
    let cutoff = most_recent_ts - window_ns;

    // Events come newest-first. Walk forward (into the past) until we cross
    // the cutoff. For each file, remember the OLDEST event in the burst,
    // because its before_hash is "the state before the whole burst started."
    let mut earliest_per_file: HashMap<String, EventRow> = HashMap::new();
    for e in undoable.iter().take_while(|e| e.ts_ns >= cutoff) {
        // Newest-first iteration: later assignment wins → stores the oldest.
        earliest_per_file.insert(e.path.clone(), (*e).clone());
    }

    let mut planned: Vec<(String, EventRow)> = earliest_per_file.into_iter().collect();
    // Deterministic ordering for the confirm prompt.
    planned.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(planned)
}

fn is_restore_bookkeeping(attribution: &str) -> bool {
    matches!(
        attribution,
        "agent-undo-restore" | "pre-restore" | "initial-scan"
    )
}

/// Print a file's content from the store by event id + side.
pub fn show_event(store: &Store, event_id: i64, show_before: bool, show_after: bool) -> Result<()> {
    let ev = store
        .get_event(event_id)?
        .ok_or_else(|| anyhow::anyhow!("no event #{event_id}"))?;

    // Default to after if neither flag given; to before if only --before.
    let want_before = show_before && !show_after;
    let (label, hash_opt) = if want_before {
        ("before", ev.before_hash.as_deref())
    } else {
        ("after", ev.after_hash.as_deref())
    };

    match hash_opt {
        Some(h) => {
            let bytes = store.read_blob(h)?;
            // Write raw bytes to stdout; fall back to lossy if UTF-8 fails.
            use std::io::Write;
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            if lock.write_all(&bytes).is_err() {
                println!("{}", String::from_utf8_lossy(&bytes));
            }
        }
        None => {
            bail!(
                "event #{event_id} has no {label} content ({} at this event)",
                if want_before {
                    "file did not exist"
                } else {
                    "file was deleted"
                }
            );
        }
    }
    Ok(())
}

/// Print a unified diff for a single event.
pub fn diff_event(store: &Store, event_id: i64) -> Result<()> {
    use similar::{ChangeTag, TextDiff};

    let ev = store
        .get_event(event_id)?
        .ok_or_else(|| anyhow::anyhow!("no event #{event_id}"))?;

    let before = read_blob_as_text(store, ev.before_hash.as_deref())?;
    let after = read_blob_as_text(store, ev.after_hash.as_deref())?;

    let diff = TextDiff::from_lines(&before, &after);
    println!("--- a/{} (event #{}, before)", ev.path, ev.id);
    println!("+++ b/{} (event #{}, after)", ev.path, ev.id);
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        // `change` prints with its trailing newline already attached.
        print!("{sign}{change}");
    }
    Ok(())
}

fn read_blob_as_text(store: &Store, hash: Option<&str>) -> Result<String> {
    match hash {
        Some(h) => {
            let bytes = store.read_blob(h)?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        }
        None => Ok(String::new()),
    }
}
