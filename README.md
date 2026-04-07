# agent-undo

> Ctrl-Z for AI coding agents. A 5MB binary that snapshots every file your AI touches and lets you roll back any edit, any session, any time.

**Status:** pre-alpha, planning phase. Target: v1 launch in 4 weeks.

## The problem

Every developer using Cursor / Claude Code / Cline / Aider / Codex has had this happen:

- Agent rewrites a file you spent hours on
- Agent "fixes" something by deleting half your function
- Agent goes wild across 8 files in 30 seconds and you can't tell what it touched
- You hit accept-all and watched your code dissolve
- You restart your IDE and the in-memory undo history is gone

Cursor's official fix for this in March 2026 was *"close the Agent Review Tab."* Their forum is full of users mocking it. Claude Code has no first-class undo. Aider only protects Aider edits via auto-commit. Cline's checkpoints are VSCode-bound. There is **no editor-agnostic, dead-simple safety net** for AI file edits.

## What agent-undo does

A single Rust binary that runs as a tiny background daemon in your project. It:

1. **Snapshots every file write** into a content-addressable store (like git, but for inter-save state)
2. **Attributes each write** to a specific agent (Claude Code, Cursor, Cline, Aider, Codex, or you)
3. **Groups edits into sessions** so an agent's "refactor across 5 files" is one undo unit
4. **Gives you one command to recover** when something goes wrong

```bash
# install
curl -fsSL https://agent-undo.dev/install.sh | sh

# in any project
agent-undo init

# the panic button
agent-undo oops
```

That's the whole UX. The rest is power-user territory.

## The killer command

```
$ agent-undo oops
⚠  Last agent action: claude-code, session 14:32-14:34, edited 5 files
   src/auth.rs        (-87 lines, +12)
   src/middleware.rs  (-23 lines, +5)
   src/lib.rs         (-4 lines, +1)
   tests/auth.rs      (deleted)
   Cargo.toml         (-2 lines, +0)

Roll back this entire session? [Y/n] _
```

One word, one keystroke, your code is back. **This is the demo GIF and it is the entire launch.**

## CLI surface

```
agent-undo init                    # set up .agent-undo/, start daemon
agent-undo status                  # daemon health, what's being watched
agent-undo log                     # event timeline (--agent, --since, --file filters)
agent-undo sessions                # list agent sessions
agent-undo diff <event-id>         # diff for one event
agent-undo diff --session <id>     # full diff of an entire session, like a PR
agent-undo show <event-id>         # print file content at point in time
agent-undo restore <event-id>      # restore file to state at event
agent-undo restore --session <id>  # roll back an entire agent session

agent-undo oops                    # ⭐ panic button: undo last agent action

agent-undo pin <label>             # pin current state, never GC
agent-undo blame <file>            # like git blame, but per-line agent attribution (v2)
agent-undo tui                     # interactive timeline scrubber
agent-undo exec --agent X -- <cmd> # attribute all writes from <cmd> to agent X
agent-undo gc                      # garbage collect old events
```

## Why this isn't a duplicate

The competitive landscape (April 2026):

- **Cursor checkpoints**: editor-bound, *demonstrably broken* per Cursor's own forum
- **Claude Code**: no first-class undo; users scrape `~/.claude` session logs to recover
- **Aider**: auto-commits to git, aider-only
- **Cline**: workspace snapshots, VSCode-bound
- **vibetracer** (closest OSS competitor): Rust, 20 stars, 2 weeks old, framed as "tracer" not rollback
- **claude-code-rewind**: 23 stars, Python, abandoned 7 months
- **All other GitHub projects**: <5 stars, weekend experiments, abandoned

No project combines: **single Rust binary + editor-agnostic + multi-agent attribution + one-command panic UX**. That's the wedge.

See `RESEARCH.md` for the full competitive map.

## Project docs

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — technical design (the daemon, attribution layers, storage, plugin system)
- [`LAUNCH.md`](LAUNCH.md) — the virality plan, week-by-week
- [`RESEARCH.md`](RESEARCH.md) — competitive landscape + naming research

## Status

- [ ] Domain registered (agent-undo.dev)
- [ ] Validation: Twitter poll + landing page
- [ ] Week 1: core daemon (FS watcher → CAS → SQLite)
- [ ] Week 2: restore + attribution + `oops`
- [ ] Week 3: Claude Code shim + TUI + polish
- [ ] Week 4: launch (HN Show + Twitter + Reddit)
