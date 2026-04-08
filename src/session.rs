use anyhow::{Context, Result};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::hook::{read_active_session, write_active_session, ActiveSession};
use crate::store::{SessionRow, Store};

#[derive(Debug, Clone)]
pub struct SessionStart {
    pub session_id: Option<String>,
    pub agent: String,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub metadata: Option<String>,
    pub tool_name: Option<String>,
    pub intended_file: Option<String>,
    pub activate: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedSessionMetadata {
    pub session_id: Option<String>,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub tool_name: Option<String>,
    pub intended_file: Option<String>,
    pub raw: Option<String>,
}

pub fn parse_metadata(raw: Option<&str>) -> Result<ParsedSessionMetadata> {
    let Some(raw) = raw else {
        return Ok(ParsedSessionMetadata::default());
    };

    let value: Value = serde_json::from_str(raw)
        .context("`--metadata` must be valid JSON, for example '{\"prompt\":\"refactor auth\"}'")?;

    Ok(ParsedSessionMetadata {
        session_id: value
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        prompt: value
            .get("prompt")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        model: value
            .get("model")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        tool_name: value
            .get("tool_name")
            .or_else(|| value.get("tool"))
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        intended_file: value
            .get("intended_file")
            .or_else(|| value.get("file_path"))
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        raw: Some(value.to_string()),
    })
}

pub fn start(store: &Store, start: SessionStart) -> Result<String> {
    let ts_ns = now_ns();
    let session_id = start
        .session_id
        .unwrap_or_else(|| format!("session-{}", Uuid::new_v4().simple()));

    store.upsert_session(&SessionRow {
        id: session_id.clone(),
        agent: start.agent.clone(),
        started_at_ns: ts_ns,
        ended_at_ns: None,
        prompt: start.prompt,
        model: start.model,
        metadata: start.metadata,
    })?;

    if start.activate {
        write_active_session(
            &store.paths,
            Some(&ActiveSession {
                session_id: session_id.clone(),
                agent: start.agent,
                started_at_ns: ts_ns,
                tool_name: start.tool_name,
                intended_file: start.intended_file,
            }),
        )?;
    }

    Ok(session_id)
}

pub fn end(store: &Store, session_id: &str, clear_active: bool) -> Result<()> {
    store.mark_session_ended(session_id, now_ns())?;

    if clear_active {
        if let Some(current) = read_active_session(&store.paths)? {
            if current.session_id == session_id {
                write_active_session(&store.paths, None)?;
            }
        }
    }

    Ok(())
}

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}
