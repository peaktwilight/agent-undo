# Philosophy — the primitive and the mindset shift

> **git for humans. `au` for agents.**
>
> git versions human *intent*. agent-undo records agent *action*. They live alongside each other.

`agent-undo` is not "yet another dev tool." It's a **primitive for a mindset shift in software engineering**: the recognition that AI coding agents are a new kind of contributor to your codebase, and they deserve the same engineering ceremony we give to every other contributor — commits, attribution, review, rollback.

This doc captures the worldview that makes the project coherent. The README sells the tool; this sells the category.

## The category claim

For 50 years, source control has had one shape: **discrete commits**. A commit is a deliberate act. The user types `git add`, then `git commit`, and the system records what the user *meant to save*. Everything between commits is invisible. Everything not staged is lost.

That model presupposes a human, working at human speed, who knows what they want to keep. The AI era violates all three assumptions:

1. **Not a human.** Agents write to disk; humans approve diffs. The actor is no longer the committer.
2. **Not human speed.** A 5-minute Claude session can produce 200 file writes. A user could not commit fast enough to capture them all even if they tried.
3. **Not knowing what to keep.** Agents change things you didn't ask for. By the time you notice, the deliberate-act commit window has long since passed.

The industry's response so far is to pretend nothing has changed: aggressive `git commit -am "wip"` discipline, hope, and prayer. When it breaks, scrape `~/.claude` session logs like an archaeologist. This is inadequate.

**agent-undo proposes a parallel category.** Not a replacement for git, not a competitor, not a wrapper. A *complement*:

- **git versions human intent.** Discrete commits, deliberate, human-paced. Stays exactly what it has always been.
- **`au` records agent action.** Continuous capture, automatic, agent-paced. Every byte every agent writes, attributed, queryable, reversible.

You use both. You commit when you mean to. `au` runs in the background and catches everything between your commits — and everything during the agent sessions you didn't realize were happening.

`au blame src/auth.rs` reads the same way `git blame src/auth.rs` does. The author column is just different: where git tells you which human wrote each line, `au` tells you which agent (or which human) wrote each line. Both views matter. Neither is sufficient alone.

This is the philosophy shift. Source control is now a *two-system* problem.

## The mindset shift

For 50 years, software engineering ceremony — version control, code review, commits, blame, CI, audit logs — has been built around one assumption: **the writer of the code is a human you can hold accountable.**

That assumption has silently broken. In 2026, a huge share of code — often a majority — is written by AI agents. Claude Code, Cursor, Cline, Aider, Codex, Continue. These agents:

- Write files you never saw
- Refactor across boundaries without understanding intent
- Fail in ways human contributors never would (silently deleting tests, rewriting mocks to make themselves pass, removing your error handling because "the happy path works")
- Operate at 100x human speed, meaning a bad decision at 14:32:07 is 200 files deep by 14:32:42
- Leave no trace of *why* they did what they did once the chat session ends

Every tool in your engineering stack — git, your editor, your CI, your review process — was designed under the assumption that a human is typing the keys. None of them are designed for a collaborator that writes 400 lines in 8 seconds while you stare at the screen trying to read fast enough.

**The industry's current response** is to pretend nothing has changed. Accept-all in Cursor. `git commit -am "claude edits"` every few minutes. Pray. When it breaks, scrape `~/.claude` session logs like an archaeologist.

**agent-undo's position**: that response is inadequate and the right answer is to build the missing primitives. Agents need:

1. **Observability** — every edit they make must be captured, always, with zero friction
2. **Attribution** — every change must be traceable to the agent, model, session, and prompt that caused it
3. **Reversibility** — any edit, any session, must be undoable as an atomic operation
4. **Accountability** — humans must be able to ask "who wrote this, when, and why" with the same ease as `git blame`
5. **Review surface** — agent sessions should be reviewable as units, the way PRs are reviewable as units

`agent-undo` is the substrate on which those primitives are built. v1 delivers observability, attribution, and reversibility. v2 adds review surface (`agent-undo blame`, session-as-PR diff). v3+ enables the full "agent-aware engineering" ecosystem — anomaly detection, compliance, team forensics.

## Why this is a primitive, not a feature

A tool is something you use. A primitive is something you *build on*. The test is whether other tools can compose on top of it.

agent-undo is designed as a primitive from day one:

- **A content-addressable store of file state over time** is a building block. Other tools can index it, diff it, replay it, export it, audit it.
- **A session-aware attribution layer** is a building block. Any tool that wants to answer "which agent touched this" calls the API.
- **A unix-socket JSON API** makes it trivially composable from any language, any editor, any workflow.
- **Plugin hooks** (v2) let the community ship `agent-undo-slack-notify`, `agent-undo-cloud-backup`, `agent-undo-pre-restore-test`, etc. without touching the core.
- **An `agentstate.db` schema** that's stable and open means other projects can read it directly, the way tools read `.git/`.

Contrast with a feature: Cursor's checkpoint button is a feature. It lives inside Cursor, it only sees Cursor-initiated edits, it can't be queried by anything else, and it breaks when Cursor breaks. It is not composable.

**The ambition**: `agent-undo` becomes to AI-agent edits what `git` is to human edits — the infrastructure layer everyone else builds on, invisible when it works, impossible to remove once adopted.

That's the primitive framing. It shapes every design decision: the schema is stable, the API is documented, the storage is open, the binary is embeddable, the daemon is scriptable.

## Principles

Engraved on a metaphorical wall. Every design decision defers to these:

### 1. The agent is an untrusted process.

Treat AI coding agents the way a security engineer treats any process with write access to your filesystem: assume it will eventually do something wrong, and build the controls that let you recover when it does. This isn't cynicism — it's the same hygiene we apply to any automated system. CI runners, build scripts, deploy bots — we monitor them because we know they can fail. Coding agents are no different.

### 2. Capture everything, delete nothing (until GC).

Storage is cheap. Lost work is catastrophic. Snapshot aggressively, garbage collect lazily, never drop data you might need in the next 10 minutes. The whole point of the tool is that the moment you need it, the data is already there.

### 3. Zero friction or zero adoption.

If it needs more than one command to install, a config file to start, or a signup to use — it will not be adopted, full stop. Dev tools that go viral in 2026 have install commands that fit in a tweet. That's not a nice-to-have; it's the price of entry.

### 4. Local-first, always.

Your code never leaves your machine. Your edit history never leaves your machine. Your agent prompts never leave your machine. There is no cloud, no telemetry, no account, no opt-out because there's nothing to opt out of. This is a non-negotiable identity — it's what makes the security story credible and the install story simple.

### 5. Never destroy data to recover data.

Every restore creates a new snapshot first. Undo the undo is always one command away. The tool must be safe in a panic. If the tool could make things worse in the moment the user needs it most, it fails its only real promise.

### 6. Memorable, not descriptive.

`agent-undo oops` is better than `agent-undo restore --session $(agent-undo sessions --last --format id)`. The tool is used in moments of stress. The common case must be one word, one keystroke, one confirmation. Everything else is power-user surface.

### 7. Editor-agnostic is the moat.

Every editor-specific "checkpoint" feature has the same ceiling: it only sees its own edits. An editor-agnostic tool sees all of them, which means it's the *only* place you can answer cross-agent questions. ("Did Claude or Cursor delete this function?" — only agent-undo can answer that.) This is the structural advantage that can't be copied by any single editor.

### 8. Expose the attribution as the killer feature.

Everyone else is building "AI undo." agent-undo is building "AI blame." The rollback is the hook; the attribution is the moat. `agent-undo blame src/auth.rs` → "line 47 written by Claude, line 48 written by you, line 49 written by Cursor" is a unique capability that only a cross-editor tool can offer. Lean into it from v2.

### 9. Composable primitives, not integrated platforms.

Don't build a dashboard. Don't build a SaaS. Don't build a web UI. Build small composable tools that do one thing and speak a common data format. The dashboards, SaaS, and web UIs will be built on top by the community — and each one will reinforce the primitive.

### 10. Ship first, then expand.

The first 1,000 users are worth more than the next 10,000. A working v1 with Claude Code support and `oops` is more valuable than a planned v2 with everything. Ship Claude Code. Launch. Let the demand from Cursor/Cline/Aider users drive v2 priorities.

## The essay that drives adoption

The philosophical framing translates directly to one blog post that should be written before launch and published the week after:

**"The AI coding agent is a new kind of contributor, and your engineering stack isn't ready for it."**

Outline:
1. The silent assumption: every tool in your stack assumes a human writes code.
2. How agents break that assumption in practice (five stories — including Jonneal3 losing 90% of his app).
3. What ceremony we give human contributors that we don't give agents (commits, blame, review, audit).
4. Why editor-specific "undo" features keep failing (they're the wrong layer).
5. The primitives we actually need: observability, attribution, reversibility, review surface.
6. How agent-undo delivers the first three today, and what comes next.
7. A call: if AI is going to write half your codebase, it should be held to half the standards we hold humans to. The tools to do that are table stakes.

This essay is the second viral wave. The launch is the hook; the essay is the category.

## The bet

If this framing is right, the category is inevitable. Someone will build it. The question is whether that someone is a funded incumbent (Cursor, Anthropic), a weekend project (vibetracer), or a cohesive Rust-native single-binary primitive that a cybersecurity-adjacent founder ships in 4 weeks and gets to 5k stars on launch day.

Being the last kind is a choice we're making. `agent-undo` exists because nobody else has made it yet, and the window is 2–4 months before someone does.
