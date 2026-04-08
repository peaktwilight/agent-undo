# LAUNCH_DRAFTS.md — copy-pasteable launch content

Everything in this file is **draft text** to be deployed on launch day. It follows the **pretext playbook** from `USE_CASES.md`:

1. Vocative letter as the launch tweet (no GIF in tweet #1)
2. Deflated, technical README opener (already done in `README.md`)
3. Demo gallery, not single hero GIF — `agent-undo.dev/demos`
4. Villain = the underlying primitive ("the agent's filesystem write"), not Cursor
5. Cursor receipts cashed in tweet #3, never in the README itself
6. Community demo flywheel via tweet #5 ("PR a rescue demo")

Replace `<NAME>` with whatever the locked project name is on launch day. Currently: **agent-undo**.

---

## Tweet #1 — the vocative launch letter

> My dear developers (and anyone who's ever lost code to an AI agent):
>
> I have built a single 3.9 MB Rust binary that snapshots every file your AI coding agent writes and lets you undo any agent session with one command. Editor-agnostic. Local-first. No cloud. The agent now has a safety net that survives even when .git is gone.
>
> agent-undo.dev

(One tweet. No GIF. Pure text + the link card preview of the README. Pretext got 19M impressions in 48 hours doing exactly this. The visuals come in tweet #2.)

---

## Tweet #2 — the hero demo

> What it actually does:
>
> [GIF — 12 seconds]
> Claude Code wrecks 5 files. User types `agent-undo oops`. Files restored byte-for-byte. Tests green. Total elapsed: 4 seconds.
>
> One word, one keystroke, your code is back.

---

## Tweet #3 — the villain receipts (this is where Cursor enters the thread)

> Why this needed to exist:
>
> Cursor staff "deanrie", Jan 2026, on the official forum bug thread "Agent code changes are automatically deleted":
>
> "This is a known issue, a bug caused by a conflict between the Agent Review Tab and file editing."
>
> Their official workaround: "Close the Agent Review Tab before the agent makes edits."
>
> [SCREENSHOT of forum thread]

---

## Tweet #4 — the moat

> The unique trick: every edit is attributed to the agent that made it.
>
> [SCREENSHOT of `agent-undo blame config.rs`]
>
> ```
> cursor       cursor-b   2026-04-08 08:33   1: pub const PORT: u16 = 9090;
> initial-scan -          2026-04-08 08:33   2: pub const HOST: &str = "localhost";
> initial-scan -          2026-04-08 08:33   3: pub const DEBUG: bool = false;
> claude-code  claude-a   2026-04-08 08:33   4: pub const TIMEOUT: u32 = 30;
> unknown      -          2026-04-08 08:33   5: pub const VERSION: &str = "1.0";
> ```
>
> Same file. Three agents. One view. No editor in the world can produce this output — only an editor-agnostic tool can see across them all.

---

## Tweet #5 — the community flywheel

> Help me build the demo gallery.
>
> Reply with the worst thing an AI agent ever did to your codebase, and I'll record an `agent-undo` demo of recovering from it. PRs to /demos welcome.
>
> Bonus points if you reproduce a real bug from your own commit history.

---

## Tweet #6 — install

> Install:
>
> ```
> curl -fsSL https://agent-undo.dev/install.sh | sh
> cd your-project
> agent-undo init --install-hooks
> ```
>
> Single 3.9 MB binary. No runtime. macOS + Linux. Windows soon.
>
> github.com/peaktwilight/agent-undo

---

## Tweet #7 — how it works in one tweet

> Under the hood:
>
> notify-rs filesystem watcher → BLAKE3 content-addressable store → SQLite timeline → Claude Code hook for attribution → `oops` command for the panic button. ~2,800 lines of Rust. <1% CPU overhead. 14 integration tests, all passing.
>
> 100% local. Zero telemetry.

---

## Tweet #8 — the philosophical anchor

> The deeper bet:
>
> AI coding agents are a new kind of contributor to your codebase. They deserve the same engineering ceremony we give human contributors — commits, attribution, review, rollback.
>
> Every editor today is failing at this. agent-undo is the missing primitive.
>
> [link to PHILOSOPHY.md]

---

## Tweet #9 — the day-2 follow-up (mirroring Cheng Lou's "cured cancer" tweet)

> Jeeesus, I wake up and apparently agent-undo cured cancer overnight. Thanks for the affection folks. Anyway, here's a 30-second screen recording of letting Claude Code run with `--dangerously-skip-permissions` for a full minute in a real codebase, then `agent-undo oops` rolling all of it back to t=0:
>
> [GIF: oops apocalypse]

---

## Tweet #10 — pinned-to-profile summary

> agent-undo: Ctrl-Z for AI coding agents.
>
> A single 3.9 MB Rust binary that snapshots every file your AI agent writes, attributes every edit to the specific agent that made it, and gives you one-command rollback when something goes wrong.
>
> agent-undo.dev | github.com/peaktwilight/agent-undo

---

## HN Show post

**Title:**
> Show HN: agent-undo – Ctrl-Z for AI coding agents (Rust, 3.9 MB binary)

**Body:**

```
Hi HN. I built agent-undo because Cursor's official fix for losing your code
to a broken agent edit, in March 2026, was "close the Agent Review Tab."
Their forum is full of users mocking it. Claude Code has no first-class undo
at all. Every editor's checkpoint feature is editor-bound and breaks under
concurrent writes.

agent-undo is a single 3.9 MB Rust binary that runs as a tiny background
daemon per project. It watches for file writes via notify-rs, hashes every
changed file with BLAKE3 into a content-addressable store, and records each
event to a SQLite timeline. When you install the Claude Code hook (one
command), every edit is attributed to the agent that made it — Claude Code,
Cursor, Cline, Aider, Codex, or you.

When something goes wrong:

    agent-undo oops

restores every file the last agent burst touched, atomically. The rollback
is itself snapshotted, so undo-the-undo is one command away.

The unique feature is `agent-undo blame <file>`, which shows per-line agent
attribution across multiple agents on the same file. No editor in the world
can produce that view, because no editor sees other editors' edits.

Architecture:
- Watch:    notify-rs FS events
- Snapshot: BLAKE3 + content-addressable .agent-undo/objects/
- Timeline: SQLite (.agent-undo/timeline.db)
- Attribute: Claude Code stdin-JSON hook protocol
- Recover:  restore, oops, blame queries over the timeline

Local-first, zero telemetry, no signup, one binary. ~2,800 lines of Rust,
14 integration tests, clippy -D warnings clean, CI on Linux + macOS.

Install:
    curl -fsSL https://agent-undo.dev/install.sh | sh

Source: github.com/peaktwilight/agent-undo
Docs:   agent-undo.dev

Happy to answer questions about the architecture, the attribution layer
design, or why I picked the specific dependency stack.
```

---

## r/cursor / r/ClaudeAI / r/LocalLLaMA post

**Title:** I built the rollback Cursor should have shipped (Rust, 3.9 MB binary)

**Body:**

```
Posting because I lost three days of work to Cursor's "Revert to Checkpoint
Broken" bug last month and I'm sick of it. The official workaround on the
Cursor forum, from a Cursor employee, is literally "close the Agent Review
Tab before the agent makes edits." That's not a fix, it's asking us to not
use the feature.

So I built agent-undo. It's a single Rust binary that:

- watches your project, snapshots every file write before it can be lost
- tags every edit with which agent did it (Claude Code, Cursor, Cline, etc)
- gives you `agent-undo oops` — one command to roll back the last agent burst

Editor-agnostic. Local-first. Zero telemetry. ~$0 cost forever.

Demo: [link to GIF]
Source: github.com/peaktwilight/agent-undo
Install: curl -fsSL https://agent-undo.dev/install.sh | sh

Looking for feedback from anyone who's been burned by editor data-loss bugs.
Specifically curious whether the Cursor forum thread workaround applies to
your setup, and whether agent-undo's `oops` recovers what you lost.
```

---

## "Why this exists" blog post outline

Title: **"The AI coding agent is a new kind of contributor, and your engineering stack isn't ready for it."**

Hook: Open with **Jonneal3's quote** from `VILLAIN.md` ("90% of my app is gone"). Then the **deanrie quote**. Then a sentence: "This is a system design failure, not a bug."

Sections:
1. The silent assumption — every tool in your stack assumes a human writes code
2. How agents break the assumption in practice (5 short stories from VILLAIN.md)
3. What ceremony we give human contributors that we don't give agents (commits, blame, review, audit)
4. Why editor-specific "undo" features keep failing (they're at the wrong layer)
5. The primitives we actually need: observability, attribution, reversibility, review surface
6. How agent-undo delivers the first three today, and what comes next
7. A call: if AI is going to write half our code, it should be held to half our standards

Publish 7 days after launch. Goal: HN front page round 2.

---

## "How agent-undo's process attribution actually works" deep-dive

Title: **"Process attribution in 2026: Layer 1 (heuristic), Layer 2 (hooks), Layer 3 (eBPF)"**

Audience: Rust + dev-tools Twitter, infosec adjacent.

Sections:
1. The problem — FSEvents, inotify, RDCW don't tell you the writing PID
2. Layer 0: zero-config heuristic via /proc + lsof (mention sysinfo crate)
3. Layer 1: polled process scanning, fingerprinting known agents
4. Layer 2: cooperative session tags via Claude Code hooks (stdin JSON protocol)
5. Layer 3: kernel-level via eBPF / EndpointSecurity (future)
6. How agent-undo composes them with confidence levels
7. The CAS + SQLite timeline schema and why we picked it

Publish 10-14 days after launch. Goal: rust-lang weekly newsletter pickup.

---

## Email capture autoresponder (for the early-access form on agent-undo.dev)

Subject: Welcome to agent-undo

```
Thanks for signing up.

agent-undo launches publicly the week of [DATE]. You'll get one email
when it ships, with the install command, the demo gallery, and the link
to the source. No other emails ever — this list is single-purpose.

In the meantime, two things:

1. If you've ever lost code to an AI agent, hit reply and tell me the
   story. I'm collecting them for the launch blog post and will credit
   you (or anonymize, your choice).

2. The source is private during dev but the README is public:
   https://github.com/peaktwilight/agent-undo

— [your name]
```

---

## Pre-launch sanity checklist

48 hours before:

- [ ] Domain DNS propagated (`agent-undo.dev`)
- [ ] Landing site deployed and Lighthouse 95+
- [ ] Demo GIF #1 (5-file Claude wreck → oops) recorded, <2 MB
- [ ] Demo GIF #2 (`oops apocalypse` 30-second version) recorded
- [ ] Demo GIF #3 (`agent-undo blame` screenshot) prepared
- [ ] OG image and Twitter card image at /og-image.png, /twitter-card.png
- [ ] Homebrew tap published with `agent-undo` formula
- [ ] crates.io package live (`cargo publish` succeeded)
- [ ] First GitHub release tagged (`v0.1.0`) with binaries attached
- [ ] `install.sh` endpoint live at agent-undo.dev/install.sh and tested
- [ ] 3 friendly devs briefed and lined up to engage in the first hour
- [ ] Twitter profile updated with pinned tweet placeholder
- [ ] Repo flipped from private to public
- [ ] CI green on main, all tests passing

Day-of:

- [ ] Tuesday 8:00 AM ET — post Tweet #1
- [ ] +5 min — post Tweet #2 with the GIF
- [ ] +10 min — post Tweet #3 with the Cursor receipts
- [ ] +15 min — post Tweet #4 with the blame screenshot
- [ ] +20 min — post Tweet #5 community-demo prompt
- [ ] +25 min — post Tweets #6, #7, #8
- [ ] Tuesday 8:30 AM ET — submit HN Show post
- [ ] Tuesday 9:00 AM ET — post r/cursor + r/ClaudeAI + r/LocalLLaMA + r/rust
- [ ] Engage every comment in first 6 hours
- [ ] Patch any install bugs immediately

Day +1:

- [ ] Post Tweet #9 (the deflated follow-up)
- [ ] Reply to outstanding HN comments
- [ ] Submit to This Week in Rust, Console.dev, Bytes newsletter
