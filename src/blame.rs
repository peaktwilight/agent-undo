// blame.rs — per-line agent attribution, the "unique moat" feature.
//
// `agent-undo blame <file>` walks the file's event history chronologically
// and replays every diff, tagging each surviving line with the agent and
// session that last touched it. The output looks like `git blame` but the
// author column is the AI agent that wrote the line, not a git committer.
//
// Algorithm:
//   1. Fetch every event for the file in chronological order.
//   2. Start with an empty vector of (line_text, attribution, ts, session).
//   3. For each event, build a line-level diff between before_hash and
//      after_hash. Apply insertions/deletions/equalities to the vector,
//      copying forward existing attributions for equal lines and stamping
//      new insertions with the event's attribution.
//   4. Print the final vector, one row per current line, with the author,
//      session id (short), timestamp, and the line text.
//
// Precision is good-enough, not surgical. We diff by line, not by token.
// That's the same resolution as git blame. A line that survives many edits
// is attributed to whoever last inserted or re-inserted it.

use anyhow::{anyhow, Result};
use chrono::{Local, TimeZone};
use similar::{ChangeTag, TextDiff};

use crate::store::Store;

#[derive(Debug, Clone)]
struct AnnotatedLine {
    text: String,
    attribution: String,
    session_id: Option<String>,
    ts_ns: i64,
}

pub fn blame(store: &Store, rel_path: &str) -> Result<()> {
    let text = blame_text(store, rel_path)?;
    print!("{text}");
    Ok(())
}

pub fn blame_text(store: &Store, rel_path: &str) -> Result<String> {
    // Pull every event for this path, oldest first.
    let mut stmt = store.conn.prepare(
        "SELECT id, ts_ns, path, before_hash, after_hash, size_before, size_after, attribution, session_id
         FROM events
         WHERE path = ?1
           AND attribution NOT IN ('pre-restore')
         ORDER BY ts_ns ASC, id ASC",
    )?;
    let events = stmt
        .query_map([rel_path], |row| {
            Ok(crate::store::EventRow {
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
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if events.is_empty() {
        return Err(anyhow!("no history for {rel_path}"));
    }

    let mut lines: Vec<AnnotatedLine> = Vec::new();
    for ev in &events {
        let before_text = match ev.before_hash.as_deref() {
            Some(h) => String::from_utf8_lossy(&store.read_blob(h)?).into_owned(),
            None => String::new(),
        };
        let after_text = match ev.after_hash.as_deref() {
            Some(h) => String::from_utf8_lossy(&store.read_blob(h)?).into_owned(),
            None => String::new(),
        };

        lines = apply_diff(
            &lines,
            &before_text,
            &after_text,
            &ev.attribution,
            ev.session_id.as_deref(),
            ev.ts_ns,
        );
    }

    Ok(render_blame(&lines))
}

fn apply_diff(
    current: &[AnnotatedLine],
    _before: &str,
    after: &str,
    attribution: &str,
    session_id: Option<&str>,
    ts_ns: i64,
) -> Vec<AnnotatedLine> {
    // We don't trust that our current attribution vector is byte-identical to
    // the event's `before` text (small races during restore could diverge).
    // So we diff the *current attribution lines* against the *after text*
    // using similar's line differ. Equal lines keep their attribution, new
    // lines get stamped with this event's.
    let current_text: String = current
        .iter()
        .map(|l| l.text.clone())
        .collect::<Vec<_>>()
        .join("");
    let diff = TextDiff::from_lines(current_text.as_str(), after);

    let mut out: Vec<AnnotatedLine> = Vec::new();
    let mut src_idx: usize = 0;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                if let Some(existing) = current.get(src_idx) {
                    out.push(existing.clone());
                } else {
                    out.push(AnnotatedLine {
                        text: change.value().to_string(),
                        attribution: attribution.to_string(),
                        session_id: session_id.map(String::from),
                        ts_ns,
                    });
                }
                src_idx += 1;
            }
            ChangeTag::Delete => {
                src_idx += 1;
            }
            ChangeTag::Insert => {
                out.push(AnnotatedLine {
                    text: change.value().to_string(),
                    attribution: attribution.to_string(),
                    session_id: session_id.map(String::from),
                    ts_ns,
                });
            }
        }
    }

    out
}

fn render_blame(lines: &[AnnotatedLine]) -> String {
    if lines.is_empty() {
        return "(file is empty)\n".into();
    }

    // Width alignment pass.
    let agent_w = lines.iter().map(|l| l.attribution.len()).max().unwrap_or(0);
    let session_w = lines
        .iter()
        .map(|l| l.session_id.as_deref().map(|s| s.len().min(8)).unwrap_or(1))
        .max()
        .unwrap_or(1);

    let mut out = String::new();
    for (n, line) in lines.iter().enumerate() {
        let ts = Local
            .timestamp_nanos(line.ts_ns)
            .format("%Y-%m-%d %H:%M")
            .to_string();
        let session = line
            .session_id
            .as_deref()
            .map(|s| &s[..s.len().min(8)])
            .unwrap_or("-");
        let text = line.text.trim_end_matches('\n');
        out.push_str(&format!(
            "{agent:<agent_w$}  {session:<session_w$}  {ts}  {n:>5}: {text}",
            agent = line.attribution,
            session = session,
            ts = ts,
            n = n + 1,
            text = text,
            agent_w = agent_w,
            session_w = session_w,
        ));
        out.push('\n');
    }
    out
}
