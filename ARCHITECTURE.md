# Architecture

This is the technical design for `agent-undo` v1. The goal is the smallest possible system that delivers the killer UX: **one binary, one command (`agent-undo oops`), zero config, perfect rollback of the last agent action**.

## Design principles

1. **Single static Rust binary.** No runtime, no Python, no Docker, no Node. Post-LiteLLM-supply-chain-attack era — minimal attack surface is a feature.
2. **Zero config to start.** `agent-undo init` should work in any project, immediately. Power-user config exists but is never required.
3. **Editor-agnostic by default.** Heuristic attribution works without integrations. Shims make it perfect.
4. **Never destroy data.** Every restore creates a new snapshot first. Undo the undo is always one command away.
5. **Sub-1% CPU overhead.** Async hashing, debounced events, content-addressable dedup, GC. Devs will rip out anything that slows their editor.
6. **Expandable, not over-engineered.** Plugin hooks for v2, but v1 is monolithic.

## High-level architecture

```
┌──────────────┐                      ┌──────────────────────┐
│   CLI        │ ◄── unix socket ────►│   agent-undo daemon  │
│  oops, log,  │                      │                      │
│  diff, tui   │                      │  ┌────────────────┐  │
└──────────────┘                      │  │  FS Watcher    │◄─┼── notify-rs
                                      │  └────────┬───────┘  │     (FSEvents/inotify/RDCW)
┌──────────────┐                      │           ▼          │
│  Shims       │ ──── tag events ────►│  ┌────────────────┐  │
│  (claude     │                      │  │ Event Pipeline │  │
│   code hook, │                      │  │  + debouncer   │  │
│   cursor ext)│                      │  └────────┬───────┘  │
└──────────────┘                      │           ▼          │
                                      │  ┌────────────────┐  │
                                      │  │ Hasher (BLAKE3)│  │
                                      │  └────────┬───────┘  │
                                      │           ▼          │
                                      │  ┌────────────────┐  │
                                      │  │ CAS Object Store│ │  .agent-undo/objects/
                                      │  │  (zstd)        │  │
                                      │  └────────┬───────┘  │
                                      │           ▼          │
                                      │  ┌────────────────┐  │
                                      │  │ Timeline       │  │  .agent-undo/timeline.db
                                      │  │  (SQLite)      │  │
                                      │  └────────────────┘  │
                                      │  ┌────────────────┐  │
                                      │  │ Attribution    │  │  process introspection
                                      │  │   Engine       │  │  + active session tags
                                      │  └────────────────┘  │
                                      └──────────────────────┘
```

One daemon per project (v1). Auto-starts on `agent-undo init`. CLI talks over a unix socket at `.agent-undo/daemon.sock`.

## Problem 1: Capturing the "before" state

You can't intercept writes from FSEvents — by the time you get the event, the file is already changed. The trick: **the "before" of every event is just the "after" of the previous event**, as long as you maintain a continuous shadow copy from the start.

The pipeline:

1. **On `agent-undo init`**: walk the project (respecting `.gitignore`), hash every file with BLAKE3, store each blob in `.agent-undo/objects/`, record initial state.
2. **On every FS event**: re-hash the file. If hash changed, write new blob, append `(path, before_hash, after_hash, ts)` to timeline.
3. **The "before_hash" is whatever the hash was last time** — already in the store.

**Editor write patterns:** Vim, JetBrains, others write to `.swp`/`.tmp` then rename. Naive watchers see CREATE→WRITE→RENAME as 3 events. Coalesce within 100ms into one logical edit on the destination.

**Skipped by default**: `.gitignore` matches, `.agent-undo/` itself, files >100MB, common build outputs (`target/`, `node_modules/`, `dist/`, `build/`). Configurable via `.agent-undoignore`.

## Problem 2: Attribution — who wrote the file

The hard part. FSEvents/inotify/RDCW don't tell you the writing PID. Three layers, increasing accuracy:

### Layer 1: Heuristic attribution (default, zero config)

When an event fires, the daemon does best-effort process introspection:

- List processes whose CWD is inside the project root (`/proc/*/cwd` on Linux, `lsof -d cwd` on macOS)
- List processes with recent open file handles in the project (`lsof` / `/proc/*/fd`)
- Match against built-in fingerprint list of known agents:
  - `claude` → `claude-code`
  - `cursor`, `cursor-helper`, `Cursor Helper` → `cursor`
  - `code` + `cline` extension → `cline`
  - `aider` → `aider`
  - `codex` → `codex`
  - `node` + `continue.continue` → `continue`
  - default → `unknown`

This is correct most of the time because in any 100ms window only one writer is active. Wrong attribution doesn't break the tool — it just shows "unknown" or guesses a sibling agent. Layers 2 and 3 fix this.

### Layer 2: Active session tags (opt-in, perfect attribution)

The daemon exposes a session API over the unix socket:

```bash
agent-undo session start --agent claude-code --metadata '{"prompt":"refactor auth"}'
# → returns session-id abc123

# ... agent does its work ...

agent-undo session end abc123
```

While a session is active, **all writes from that PID and its children are tagged with that session**. Shims call this:

- **Claude Code** (native fit): `~/.claude/settings.json` hooks. **Important:** Claude Code hooks do not expose per-call metadata via env vars — they write JSON to the hook process's **stdin**. Schema:

  ```json
  {
    "session_id": "abc123",
    "tool_name": "Write",
    "tool_input": { "file_path": "/abs/path.rs", "content": "..." },
    "tool_response": { "success": true, "filePath": "/abs/path.rs" }
  }
  ```

  (`tool_response` only present on PostToolUse.) The matcher field is a regex; `"Write|Edit|MultiEdit|NotebookEdit"` is the idiom. Installed hook:

  ```json
  {
    "hooks": {
      "PreToolUse": [{
        "matcher": "Write|Edit|MultiEdit|NotebookEdit",
        "hooks": [{
          "type": "command",
          "command": "agent-undo hook pre"
        }]
      }],
      "PostToolUse": [{
        "matcher": "Write|Edit|MultiEdit|NotebookEdit",
        "hooks": [{
          "type": "command",
          "command": "agent-undo hook post"
        }]
      }]
    }
  }
  ```

  `agent-undo hook pre|post` reads the JSON from stdin, extracts `session_id` / `tool_name` / `tool_input.file_path`, and calls the daemon over the unix socket to tag the session.

  **Installer must merge, not replace** the existing hooks arrays — users likely already have entries. Load order across settings files (user → project → local) is merge-based, so simply appending is correct.

  `agent-undo init` auto-installs this with user consent.

- **Cursor**: tiny VSCode marketplace extension that calls the API on each chat turn (v2)
- **Cline**: similar VSCode extension or PR upstream
- **Aider / Codex / Continue**: PR upstream hook points
- **Generic**: `agent-undo exec --agent X -- <cmd>` wraps any command and attributes its writes

The shims are *optional*. Without them, heuristic attribution still works. With them, attribution is perfect and includes the prompt that caused the edit.

### Layer 3: Kernel-level attribution (v3, opt-in)

For users who want bulletproof attribution:
- **Linux**: eBPF probe on `vfs_write` capturing PID, comm, path
- **macOS**: EndpointSecurity (requires entitlement + signed helper)

Skip for v1. Layers 1 + 2 are enough.

## Problem 3: Storage layout

```
.agent-undo/
├── config.toml
├── objects/                  ← content-addressable store
│   ├── ab/
│   │   └── cdef012345...     ← BLAKE3 hash → raw file bytes (zstd compressed)
│   └── ...
├── timeline.db               ← SQLite
├── sessions/
│   └── abc123.json           ← session metadata
└── daemon.sock               ← unix socket (Linux/macOS)
```

**SQLite schema:**

```sql
CREATE TABLE events (
  id            INTEGER PRIMARY KEY,
  ts_ns         INTEGER NOT NULL,
  path          TEXT NOT NULL,
  before_hash   TEXT,           -- null if file created
  after_hash    TEXT,           -- null if file deleted
  size_before   INTEGER,
  size_after    INTEGER,
  attribution   TEXT NOT NULL,  -- 'human' | 'claude-code' | 'cursor' | ...
  confidence    TEXT NOT NULL,  -- 'high' | 'medium' | 'low' | 'none'
  session_id    TEXT,
  pid           INTEGER,
  process_name  TEXT,
  tool_name     TEXT,           -- 'Edit', 'Write', 'MultiEdit', etc.
  metadata      JSON
);

CREATE INDEX idx_events_path_ts ON events(path, ts_ns DESC);
CREATE INDEX idx_events_session ON events(session_id);
CREATE INDEX idx_events_agent_ts ON events(attribution, ts_ns DESC);

CREATE TABLE sessions (
  id            TEXT PRIMARY KEY,
  agent         TEXT NOT NULL,
  started_at_ns INTEGER NOT NULL,
  ended_at_ns   INTEGER,
  prompt        TEXT,
  model         TEXT,
  metadata      JSON
);

CREATE TABLE pins (
  id            INTEGER PRIMARY KEY,
  event_id      INTEGER NOT NULL,
  label         TEXT NOT NULL,
  created_at_ns INTEGER NOT NULL
);
```

**Dedup:** identical content → identical BLAKE3 hash → one blob. Edit a 1MB file 100 times with small changes → ~1MB total storage.

**Reflinks** on APFS / btrfs / XFS where available — snapshots cost zero disk for unchanged files.

**Garbage collection** in `config.toml`:
```toml
[gc]
keep_last        = "7d"
keep_per_session = "all"
keep_session_for = "30d"
max_size_gb      = 5
```

`agent-undo gc` runs manually or on schedule.

## Problem 4: Restoration

CLI surface (designed for the moment of panic — `oops` is the only one that matters at first):

```
agent-undo oops                       # interactive: undo last agent action
agent-undo oops --confirm             # skip prompt
agent-undo log [--agent X] [--since 1h]
agent-undo sessions
agent-undo diff <event-id>
agent-undo diff --session <id>
agent-undo show <event-id> [--before|--after]
agent-undo restore <event-id>
agent-undo restore --file F --to <ts>
agent-undo restore --session <id>     # transactional multi-file rollback
agent-undo pin <label>
agent-undo blame <file>               # v2: per-line agent attribution
agent-undo tui                        # ratatui timeline browser
agent-undo exec --agent X -- <cmd>
agent-undo gc
agent-undo session start/end          # for shims
```

**Critical safety rule:** every restore operation **first creates a snapshot of current state** tagged `attribution = "agent-undo-restore"`. Restores are always reversible.

**Transactional multi-file restore:** when an agent edits 5 files in a session, `agent-undo restore --session xyz` rolls back all 5 atomically. This is the killer feature — agent edits are treated as the unit of meaningful change, not individual file writes.

## Performance budget

- **Idle CPU**: <0.1% (just notify-rs polling)
- **Active CPU during edits**: <1% on a normal coding session
- **Hash latency**: BLAKE3 ~6 GB/s; a 100KB file takes ~17µs
- **SQLite write latency**: ~100µs per event with WAL mode
- **Disk overhead**: ~1.5x project size for first week, plateaus after GC

## Dependencies (Rust crates)

- `tokio` — async runtime
- `notify` — cross-platform FS events
- `blake3` — fast hashing
- `rusqlite` (bundled) — SQLite
- `zstd` — blob compression
- `clap` — CLI parsing
- `ratatui` + `crossterm` — TUI
- `serde` + `serde_json` — config + IPC
- `ignore` — `.gitignore` parsing (same crate ripgrep uses)
- `sysinfo` — process introspection
- `dirs` — XDG paths
- `axum` — unix socket server (or just raw `tokio::net::UnixListener`)

Single static binary, ~5–8MB stripped. No system dependencies.

## Plugin system (v2 design, not in v1)

Subprocess plugins via JSON-RPC over stdin/stdout. Language-agnostic. No WASM runtime to ship.

```toml
[plugins]
post_event   = ["agent-undo-slack-notify", "/usr/local/bin/my-hook"]
attribution  = "agent-undo-fancy-attrib"
pre_restore  = ["agent-undo-confirm-with-team"]
```

Example post_event payload:
```json
{
  "type": "post_event",
  "event": {
    "id": 4823,
    "path": "src/auth.rs",
    "attribution": "claude-code",
    "session_id": "abc123",
    "ts_ns": 1712503284000000000
  }
}
```

Plugin opportunities for the community:
- `slack-notify` — ping when an agent rewrites >100 lines
- `cloud-backup` — sync timeline to S3
- `pre-restore-test` — run `cargo test` before allowing a restore
- `blame-export` — per-PR attribution reports
- `anomaly-detect` — flag agent edits matching suspicious patterns (the security angle)

## What's NOT in v1

Explicitly deferred to keep scope tight:

- ❌ MCP server mode
- ❌ LD_PRELOAD / DYLD_INSERT_LIBRARIES / eBPF / EndpointSecurity
- ❌ Web UI / dashboard
- ❌ Cloud sync / team mode
- ❌ Branch / experiment mode (parallel agents, pick winner)
- ❌ Semantic diff via LLM
- ❌ `agent-undo blame` (line-level attribution) — v2
- ❌ Cursor / Cline / Aider / Codex shims — only Claude Code in v1, others in v2
- ❌ Plugin system — v2
- ❌ Windows support — Linux + macOS in v1, Windows in v2

## v1 build sequence (4 weeks)

**Week 1 — core pipeline**
- Crate scaffold
- `notify-rs` watcher → debouncer → BLAKE3 hasher → CAS → SQLite
- `agent-undo init`, `serve`, `status`, `log`
- Daemon lifecycle (auto-start, unix socket)

**Week 2 — restore + attribution**
- `diff`, `show`, `restore`
- `oops` (the panic button)
- Heuristic attribution via `sysinfo` + `lsof`
- Session API (`session start/end`)
- `exec` wrapper

**Week 3 — first integration + polish**
- Claude Code hook auto-installer
- `tui` (ratatui)
- `.gitignore` / `.agent-undoignore`
- Editor write-pattern coalescing
- GC
- Install script + Homebrew tap

**Week 4 — launch prep**
- README + 15-second demo GIF (Claude Code wrecks file → `oops` → restored)
- Benchmarks (overhead %, disk usage, p99 latency)
- Docs site (mdbook)
- HN Show post draft
- Reddit + Twitter posts ready
- 3 friendly devs lined up to engage in the first hour
