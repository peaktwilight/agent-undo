// install.rs — `agent-undo init --install-hooks` logic.
//
// The Claude Code hook installer. Patches ~/.claude/settings.json to add
// PreToolUse + PostToolUse hooks that call `agent-undo hook pre|post`.
//
// Rules:
//   - Never replace existing hook arrays. Merge by appending a new entry
//     with the agent-undo matcher so the user's existing hooks keep working.
//   - Recognize our own entries and skip if already installed (idempotent).
//   - Back up the settings.json before writing (<path>.agent-undo.bak).
//   - Fail softly if the file is malformed — warn but don't crash init.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const HOOK_MATCHER: &str = "Write|Edit|MultiEdit|NotebookEdit";
const HOOK_MARKER: &str = "agent-undo-managed";

/// Resolve `~/.claude/settings.json`. Returns None if $HOME can't be found.
pub fn claude_settings_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("settings.json"))
}

/// Install the agent-undo hooks into the Claude Code settings.json, merging
/// with whatever the user already has. Returns an (installed, path) pair —
/// `installed = false` means the hooks were already present.
pub fn install_claude_hooks() -> Result<(bool, PathBuf)> {
    let path = claude_settings_path()
        .ok_or_else(|| anyhow::anyhow!("could not resolve home directory"))?;

    // Read-or-create root object.
    let mut root: Value = if path.exists() {
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        if bytes.is_empty() {
            json!({})
        } else {
            match serde_json::from_slice(&bytes) {
                Ok(v) => v,
                Err(e) => {
                    bail!(
                        "~/.claude/settings.json is not valid JSON: {e}\n\
                         Fix it manually or move it aside before running init --install-hooks."
                    );
                }
            }
        }
    } else {
        json!({})
    };

    if !root.is_object() {
        bail!("~/.claude/settings.json root is not an object");
    }

    let hooks = root
        .as_object_mut()
        .unwrap()
        .entry("hooks")
        .or_insert_with(|| json!({}));
    if !hooks.is_object() {
        bail!("settings.json 'hooks' is not an object");
    }
    let hooks_obj = hooks.as_object_mut().unwrap();

    let mut changed = false;
    for (event, cli) in [("PreToolUse", "pre"), ("PostToolUse", "post")] {
        let arr = hooks_obj
            .entry(event.to_string())
            .or_insert_with(|| json!([]));
        let arr = match arr.as_array_mut() {
            Some(a) => a,
            None => bail!("settings.json 'hooks.{event}' is not an array"),
        };

        // Check whether we already installed our entry.
        let already = arr.iter().any(|entry| {
            entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .map(|hs| {
                    hs.iter().any(|h| {
                        h.get("__comment")
                            .and_then(|c| c.as_str())
                            .map(|s| s.contains(HOOK_MARKER))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        });
        if already {
            continue;
        }

        arr.push(json!({
            "matcher": HOOK_MATCHER,
            "hooks": [{
                "type": "command",
                "command": format!("agent-undo hook {}", cli),
                "__comment": format!("{} — do not edit", HOOK_MARKER)
            }]
        }));
        changed = true;
    }

    if changed {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if path.exists() {
            let backup = path.with_extension("json.agent-undo.bak");
            let _ = fs::copy(&path, &backup);
        }
        let pretty = serde_json::to_string_pretty(&root)?;
        let tmp = path.with_extension("json.agent-undo.tmp");
        fs::write(&tmp, pretty.as_bytes())?;
        fs::rename(&tmp, &path)?;
    }

    Ok((changed, path))
}

/// Remove agent-undo's hooks from Claude Code settings.json, leaving
/// everything else intact. Returns how many entries were removed.
#[allow(dead_code)] // wired in by v0.3 `init --uninstall-hooks`
pub fn uninstall_claude_hooks() -> Result<usize> {
    let path = match claude_settings_path() {
        Some(p) if p.exists() => p,
        _ => return Ok(0),
    };

    let bytes = fs::read(&path)?;
    if bytes.is_empty() {
        return Ok(0);
    }
    let mut root: Value = serde_json::from_slice(&bytes)?;
    let Some(hooks) = root.get_mut("hooks").and_then(|h| h.as_object_mut()) else {
        return Ok(0);
    };

    let mut removed = 0usize;
    for event in ["PreToolUse", "PostToolUse"] {
        if let Some(arr) = hooks.get_mut(event).and_then(|v| v.as_array_mut()) {
            let before = arr.len();
            arr.retain(|entry| {
                !entry
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .map(|hs| {
                        hs.iter().any(|h| {
                            h.get("__comment")
                                .and_then(|c| c.as_str())
                                .map(|s| s.contains(HOOK_MARKER))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            });
            removed += before - arr.len();
        }
    }
    if removed > 0 {
        let pretty = serde_json::to_string_pretty(&root)?;
        fs::write(&path, pretty.as_bytes())?;
    }
    Ok(removed)
}
