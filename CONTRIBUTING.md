# Contributing to agent-undo

Thanks for considering it. This file covers what you need to know to land a useful PR.

## Project shape in one paragraph

`agent-undo` is a single Rust binary. The pipeline is: `notify-rs` filesystem watcher → BLAKE3 hasher → content-addressable blob store under `.agent-undo/objects/` → SQLite timeline at `.agent-undo/timeline.db`. Attribution comes from a small JSON marker file (`.agent-undo/active-session.json`) that AI editors update via `agent-undo hook pre|post`. Restore commands are queries + inverse operations over the timeline. The main user-facing verb is `oops`.

The deeper rationale is in [`PHILOSOPHY.md`](PHILOSOPHY.md). The technical design is in [`ARCHITECTURE.md`](ARCHITECTURE.md). The launch playbook is in [`LAUNCH_DRAFTS.md`](LAUNCH_DRAFTS.md) and [`USE_CASES.md`](USE_CASES.md).

## What we welcome

- **Bug reports** with a minimal reproduction. The most helpful format: paste the output of `agent-undo log -n 20` and the exact commands you ran.
- **New editor integrations** — Cursor, Cline, Aider, Codex, Continue. Each lives as a small adapter in the [`integrations/`](integrations/) directory once that exists. Until then, propose the design in an issue first.
- **New attribution sources** — process scanning heuristics, eBPF, EndpointSecurity, LD_PRELOAD shims. See `ARCHITECTURE.md` "Layered attribution" for the design boundary.
- **Performance fixes** with measurements. We care about <1% CPU during normal coding sessions.
- **More integration tests.** The bar is: every new CLI command gets at least one test in `tests/integration.rs` that drives the built binary.
- **Documentation fixes**, especially clearer explanations of edge cases.

## What we don't want (yet)

- **New top-level CLI commands** without an issue discussing them first. The CLI surface is the launch story; we keep it tight.
- **GUI / web UI / SaaS dashboards.** This project is local-first by design and will stay that way.
- **Telemetry, analytics, or any phone-home behavior** — including opt-in. The "no telemetry" promise is non-negotiable.
- **Cloud sync features in the core.** A `agent-undo-cloud-backup` plugin can exist as a separate crate, but it never lands in the main binary.
- **Refactors without behavior changes.** Style preferences are personal; please don't reshape modules just to taste.
- **Dependency additions** without a clear cost/benefit. We're aiming for a small static binary.

## Dev setup

You need stable Rust (currently 1.94+). If you're touching `www/`, use Node
`>=22.12.0` (see [`www/.nvmrc`](www/.nvmrc)). Everything else is in the
lockfile.

```sh
git clone https://github.com/peaktwilight/agent-undo.git
cd agent-undo
cargo build
cargo test --test integration
```

The integration tests run the built binary against real temp directories — they should all pass on a fresh checkout.

## Before sending a PR

Run, in this order:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --test integration
cargo build --release
```

CI runs the same four steps on Linux + macOS, plus a focused Windows watcher/rollback smoke test. If any fail, the PR can't merge.

## Code style

- **No `unwrap()` or `expect()` in production paths.** Use `?` and propagate `anyhow::Result`. Tests can `.unwrap()` freely.
- **Comments explain *why*, not *what*.** The "what" is in the code; the "why" is the load-bearing context.
- **Module headers**: every `src/*.rs` file should open with a 5-15 line block comment explaining its responsibility, the relationships to other modules, and any non-obvious invariants. Match the existing style.
- **Errors**: prefer `anyhow::Result<T>` everywhere except library boundaries (none yet). Wrap with `.with_context(|| format!("…"))` at every fallible filesystem boundary.
- **No `Box<dyn Error>`** when `anyhow::Error` will do.
- **Naming**: `snake_case` everywhere; CLI subcommands use the same name as their `cmd_*` function.

## Commit hygiene

- Subject line: imperative, ≤72 chars, no trailing period. Example: `Add session-scoped restore + atomic multi-file rollback`.
- Body: prose paragraphs, not bullet pyramids. Explain *why* the change is shaped the way it is.
- One logical change per commit. Don't squash unrelated improvements together.
- Reference issues with `Closes #N` if applicable.
- Sign nothing. We don't gpg-sign right now.

## Testing philosophy

Integration tests > unit tests for this project. We care about end-to-end behavior of the binary. A good integration test:

1. Creates a unique temp directory.
2. Runs `agent-undo init` (and any setup commands).
3. Performs the scenario under test (file edits, hook calls, etc.).
4. Asserts on the visible output of `agent-undo log` / `sessions` / file content.
5. Cleans up the temp directory.

Look at `tests/integration.rs` for the patterns. The `unique_tmp_dir`, `bin_path`, and `run` helpers are designed to be copy-pasted into new tests.

## Security

If you find a vulnerability, please **do not open a public issue**. Email the maintainer (see the GitHub profile) or open a private security advisory via GitHub's "Security" tab.

The threat model: agent-undo handles potentially-sensitive user code on disk. The main risks we care about are:

1. **Symlink races during snapshot or restore** — we use atomic temp+rename and we never follow symlinks out of the project root. Tell us if you find a way to escape.
2. **Path traversal via crafted file paths** — we use `Path::strip_prefix` and `is_dir()` checks but we'd love a fuzz harness around this.
3. **Hook command injection** — the Claude Code hook reads JSON on stdin, never executes shell from it. The `exec` wrapper passes argv directly, no shell.

Please test against these if you're poking at the code with a security hat on.

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0 — the same license as the rest of the project. See [`LICENSE`](LICENSE).

## Code of Conduct

Be the kind of person you'd want a stranger to be when reporting a bug in your code. That's the whole policy.
