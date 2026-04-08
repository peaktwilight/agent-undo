# agent-undo

*Local-first rollback for AI coding agents. A single binary that snapshots every file your agent writes and lets you undo any session with one command.*

*agent-undo side-steps editor checkpoints, IDE history, and after-the-fact `git reflog` archaeology — all of which silently fail when the agent has been given write access to the filesystem and acted faster than your save loop.*

```sh
curl -fsSL https://agent-undo.dev/install.sh | sh
```

```sh
cd my-project
agent-undo init --install-hooks
# ... agent goes wild ...
agent-undo oops
```

---

## What it does

Every AI coding agent today writes to your filesystem the same way you would: directly, immediately, and irreversibly. Editor checkpoints are an in-memory afterthought. `git` is not a save loop. When the agent moves faster than your commit cadence, the only safety net is an out-of-band log of every byte that hit disk.

`agent-undo` is that log.

It runs as a small background daemon per project, snapshotting every file write into a content-addressable store (BLAKE3 hashes, zstd blobs, SQLite timeline). Every edit is attributed to the agent that made it — Claude Code, Cursor, Cline, Aider, Codex, or you — via a small hook that each of them can call. Nothing ever leaves your machine.

When something goes wrong, you type one word:

```sh
agent-undo oops
```

and the last burst of agent edits is rolled back, atomically, across every file that was touched. The rollback is itself recorded, so undo-the-undo is always one command away.

## Use cases

1. **Recover from a bad agent edit.** The hero use case. `agent-undo oops`.
2. **Audit what an agent actually changed.** `agent-undo log --agent claude-code --since 1h` and `agent-undo diff --session <id>`.
3. **Per-line agent attribution.** `agent-undo blame <file>` — like `git blame`, but tells you which agent (or human) wrote each line.
4. **Pin a known-good state before letting an agent loose.** `agent-undo pin "before refactor"` and restore to it later.

## Install

```sh
curl -fsSL https://agent-undo.dev/install.sh | sh
```

Or from source:

```sh
cargo install agent-undo
```

Or from the latest GitHub release: macOS (arm64, x64), Linux (x64, arm64) — single 5–8 MB binary, no runtime.

## Quick start

```sh
cd my-project
agent-undo init --install-hooks      # sets up .agent-undo/ and patches
                                      # ~/.claude/settings.json so Claude Code
                                      # edits are attributed automatically
agent-undo serve &                    # watcher loop (v0.2 daemonizes this)

# ... work normally with Claude Code / Cursor / Cline / Aider / Codex ...

agent-undo log                        # see every file event, attributed
agent-undo sessions                   # list recent agent sessions
agent-undo oops                       # undo the last burst of agent edits
```

## How it works

1. **Watch.** A `notify-rs` filesystem watcher sees every write in the project tree. `.gitignore` and `.agent-undoignore` are respected.
2. **Snapshot.** Each changed file is hashed with BLAKE3 and written into a content-addressable object store under `.agent-undo/objects/`. Identical content dedupes automatically.
3. **Attribute.** Before an agent writes, its hook (`agent-undo hook pre`) drops a small JSON marker identifying the active session. The watcher reads the marker on each event and tags the resulting timeline entry with the agent, session id, and tool name.
4. **Recover.** Every event lives in a SQLite timeline at `.agent-undo/timeline.db`. `restore`, `oops`, `diff`, and `show` are all queries and inverse operations over that table. Every restore snapshots the current state first — you can never lose data by undoing.

No cloud. No account. No telemetry. One binary. One SQLite file. Your code never leaves the machine.

## Design rules

- **The agent is an untrusted process.** Treat AI coding agents the way a security engineer treats any process with write access to your filesystem.
- **Capture everything, delete nothing (until GC).**
- **Zero friction or zero adoption.** Install in one command. Works with every major AI editor out of the box.
- **Local-first, always.**
- **Never destroy data to recover data.** Every restore creates a new snapshot first.

Longer essay: [`PHILOSOPHY.md`](PHILOSOPHY.md).

## Status

`v0.0.x` — pre-alpha. The core pipeline works end-to-end (`init`, `serve`, `log`, `sessions`, `diff`, `show`, `restore`, `oops`, `hook pre|post`, `exec`, `init --install-hooks`). 9/9 integration tests passing.

Coming next: daemon lifecycle (`serve --daemon`), `agent-undo blame`, `agent-undo tui`, Cursor / Cline / Aider hook integrations, Homebrew tap, `install.sh` endpoint.

## License

Dual-licensed under MIT and Apache-2.0.
