// hook.rs — Claude Code hook protocol handlers.
//
// Claude Code hooks don't use env vars for tool metadata — they write JSON
// on stdin. See ARCHITECTURE.md for the schema. Our job:
//
//   `agent-undo hook pre`  — runs BEFORE the tool executes. Parses
//   session_id + tool_name + file_path. Records the session in SQLite
//   and writes `.agent-undo/active-session.json` so the watcher can
//   attribute the incoming file write correctly.
//
//   `agent-undo hook post` — runs AFTER the tool executes. Marks the
//   session as "last activity at now" and clears the active-session marker.
//
// This is the Layer 2 attribution from ARCHITECTURE.md ("active session tags").
// Perfect attribution when Claude Code hooks are installed, vs. "unknown" from
// Layer 1 heuristics when they aren't.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths::ProjectPaths;
use crate::store::{SessionRow, Store};

/// JSON Claude Code writes to stdin for hook commands.
#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeHookInput {
    pub session_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: serde_json::Value,
    #[serde(default)]
    #[allow(dead_code)] // parsed for PostToolUse inspection in v0.3
    pub tool_response: Option<serde_json::Value>,
}

impl ClaudeHookInput {
    /// Extract the file path from tool_input for tools that have one.
    pub fn file_path(&self) -> Option<String> {
        self.tool_input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(String::from)
    }
}

/// What the watcher reads on each event to decide attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSession {
    pub session_id: String,
    pub agent: String,
    pub started_at_ns: i64,
    pub tool_name: Option<String>,
    pub intended_file: Option<String>,
}

pub fn handle_pre() -> Result<()> {
    let input = read_stdin_json()?;
    run_pre(input)
}

pub fn handle_post() -> Result<()> {
    let input = read_stdin_json()?;
    run_post(input)
}

fn run_pre(input: ClaudeHookInput) -> Result<()> {
    let paths = ProjectPaths::discover().ok();
    let Some(paths) = paths else {
        // No .agent-undo/ in the current tree — silently no-op so the hook
        // never blocks Claude Code just because the user hasn't initialized
        // this particular project.
        return Ok(());
    };

    let store = Store::open(paths.clone())?;
    let ts = now_ns();

    // Persist the session in SQLite so it survives daemon restarts.
    store.upsert_session(&SessionRow {
        id: input.session_id.clone(),
        agent: "claude-code".into(),
        started_at_ns: ts,
        ended_at_ns: None,
        prompt: None,
        model: None,
        metadata: Some(serde_json::json!({ "tool": input.tool_name }).to_string()),
    })?;

    // Write the active-session marker the watcher reads.
    let active = ActiveSession {
        session_id: input.session_id.clone(),
        agent: "claude-code".into(),
        started_at_ns: ts,
        tool_name: Some(input.tool_name.clone()),
        intended_file: input.file_path(),
    };
    write_active_session(&paths, Some(&active))?;
    Ok(())
}

fn run_post(input: ClaudeHookInput) -> Result<()> {
    let paths = ProjectPaths::discover().ok();
    let Some(paths) = paths else {
        return Ok(());
    };
    let store = Store::open(paths.clone())?;
    let ts = now_ns();
    store.mark_session_ended(&input.session_id, ts)?;

    // Only clear the marker if it still points at this session — don't race
    // with a later pre-hook that already installed a newer session.
    if let Some(current) = read_active_session(&paths)? {
        if current.session_id == input.session_id {
            write_active_session(&paths, None)?;
        }
    }
    Ok(())
}

pub fn active_session_path(paths: &ProjectPaths) -> std::path::PathBuf {
    paths.data_dir.join("active-session.json")
}

pub fn read_active_session(paths: &ProjectPaths) -> Result<Option<ActiveSession>> {
    let path = active_session_path(paths);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let parsed: ActiveSession =
        serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(parsed))
}

pub fn write_active_session(paths: &ProjectPaths, session: Option<&ActiveSession>) -> Result<()> {
    let path = active_session_path(paths);
    match session {
        Some(s) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let bytes = serde_json::to_vec_pretty(s)?;
            let tmp = path.with_extension("tmp");
            fs::write(&tmp, &bytes)?;
            fs::rename(&tmp, &path)?;
        }
        None => {
            if path.exists() {
                let _ = fs::remove_file(&path);
            }
        }
    }
    Ok(())
}

fn read_stdin_json() -> Result<ClaudeHookInput> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("reading hook JSON from stdin")?;
    let parsed: ClaudeHookInput =
        serde_json::from_str(&buf).context("parsing Claude Code hook JSON")?;
    Ok(parsed)
}

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}
