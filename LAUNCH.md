# Launch plan — maximizing virality

The goal: **HN Show front page + 5k GitHub stars in week one**. The research on viral OSS in 2024-2026 (Ruff, uv, Bun, Ollama, Aider, Cline, Open Interpreter) gives a clear playbook. This doc applies it specifically to `agent-undo`.

## The viral checklist (from research)

Every viral dev-tool in the last two years hits these:

| Property | agent-undo plan |
|---|---|
| Single-line install | `curl -fsSL https://agent-undo.dev/install.sh \| sh` |
| Single static binary | Rust, ~5–8MB, no runtime |
| Time-to-wow ≤ 60s | `init` → trigger an edit → `oops` → restored. ~30s. |
| Replaces a broken incumbent | Cursor checkpoints (publicly broken), Claude Code's nothing |
| Named villain | "Cursor's official fix is *close the Agent Review Tab*" |
| Benchmark / number in pitch | "<1% CPU overhead, restores in 12ms, works across 5 agents" |
| 15-second demo GIF in README first 200px | Yes — the entire launch |
| Local-first, no signup, no telemetry | Yes |
| BYO-everything | No accounts, no keys, nothing to configure |
| Editor-agnostic | Yes — that's the wedge vs Cursor/Cline checkpoints |
| Founder credibility | Cybersecurity background = "treat your agent like a compromised process" |
| Memorable verb | `oops` |

Every box checked.

## The pitch (one sentence)

> **agent-undo is a 5MB binary that snapshots every file your AI coding agent touches and lets you undo any session with one command. Editor-agnostic. Local-first. Zero config.**

## The villain

> "Cursor's official fix for losing your work to broken checkpoints in March 2026 was *close the Agent Review Tab*. Their forum is full of users mocking it. Claude Code has no first-class undo at all. agent-undo is the safety net every AI coding tool should have shipped from day one — and didn't."

This frames it as **the missing layer the editors failed to ship**, not "another tool." It's not competing with Cursor — it's *fixing what Cursor couldn't*.

## The demo GIF (the entire launch)

The 15-second screencap that runs in the README and every social post:

1. Split terminal: Claude Code on the left, file open on the right
2. User: *"Refactor src/auth.rs to use the new token format"*
3. Claude wrecks the file (deletes 80 lines, breaks the auth flow)
4. Test fails, screen flashes red
5. User types `agent-undo oops`
6. Prompt: *"Roll back claude-code session 14:32-14:34? [Y/n]"*
7. User hits Y
8. File restored, test passes, screen green
9. End frame: **agent-undo — Ctrl-Z for AI**

This GIF *is* the launch. Spend a full day making it perfect. Every viral tool in 2025-2026 has one. Aider's, Ollama's, Open Interpreter's — go study them.

## Validation step (1 day, BEFORE building)

Don't write Rust until the visceral reaction is confirmed.

**Day 0 — validation**
- Buy `agent-undo.dev`
- Build a one-page landing site (Astro or just plain HTML)
  - Headline: "Ctrl-Z for AI coding agents"
  - Subhead: the pitch
  - The demo GIF (mocked if needed — screen-record the *intended* UX)
  - Email signup form
  - "Coming in 4 weeks" footer
- Twitter poll: *"Has an AI coding agent ever destroyed work you couldn't recover?"*
- Reddit posts:
  - r/cursor: *"I'm building a Ctrl-Z for AI editors. Would you use this?"* (link to landing)
  - r/ClaudeAI, r/ChatGPTCoding, r/LocalLLaMA: same
- Hacker News: skip (save HN ammo for the real launch)

**Success criteria:**
- 200+ landing page email signups in 48h → ship it
- 50–199 → ship it but smaller scope
- <50 → reconsider

This costs one day and de-risks four weeks.

## Build sequence

See `ARCHITECTURE.md` for the engineering plan. Summary:

- **Week 1**: core pipeline (FS watcher → CAS → SQLite)
- **Week 2**: restore + `oops` + heuristic attribution
- **Week 3**: Claude Code hook + TUI + polish
- **Week 4**: launch prep

## Launch day mechanics

**Tuesday, 8:00am ET.** This is the empirically best HN slot.

**Hour -24** (Monday morning):
- Final README pass
- Demo GIF embedded, autoplay, <2MB
- Install script tested on fresh macOS + Ubuntu
- Homebrew tap live (`brew install peak/tap/agent-undo`)
- Crates.io package live (`cargo install agent-undo`)
- GitHub release v0.1.0 with binary attachments for macOS x64/arm64 + Linux x64/arm64
- mdbook docs site live at agent-undo.dev/docs
- 3 friendly devs briefed: be ready to engage substantively in first hour

**Hour 0** (Tuesday 8am ET):
- HN Show post: *"Show HN: agent-undo — Ctrl-Z for AI coding agents (Rust, 5MB binary)"*
- Twitter thread (10 tweets):
  1. The hook ("Cursor's fix for lost code is 'close the tab'. I built a better one.")
  2. The GIF
  3. The problem (Cursor forum screenshots)
  4. The solution (one binary, one command)
  5. How attribution works
  6. How to install
  7. What's next
  8. Tag: @cursor_ai, @AnthropicAI, @cline_ai, @paulgauthier, plus any AI-tool-builder accounts
  9. Link to landing
  10. "RT if you've ever lost code to an AI agent"
- Reddit posts (staggered over the day):
  - r/programming
  - r/rust (Rust community loves new tooling launches)
  - r/LocalLLaMA
  - r/cursor (high engagement, this audience IS the user)
  - r/ChatGPTCoding
  - r/ClaudeAI

**Hour 0–6**: maintainer mode
- Reply to every HN comment within minutes
- Fix any install bugs immediately and push patch releases
- Don't argue, ship fixes
- Post a "what I learned in the first 6 hours" thread for second wave

**Day 2–7**:
- Daily commits visible on the repo (momentum signal)
- Engage with every issue
- Write follow-up blog post: *"How agent-undo's process attribution actually works"* (the technically interesting deep dive — second viral wave)
- Submit to weekly newsletters: This Week in Rust, Console.dev, Bytes, AI Engineer Pack

## Second-wave content (week 2-3)

Once the initial spike fades, post substance:

1. **Technical post**: *"Building a process-attributed file watcher in 600 lines of Rust"* — details on Layer 1/2/3 attribution. Targets Rust + dev-tools audience.
2. **Security post**: *"Treating AI coding agents as untrusted processes"* — leverages cybersecurity background. Targets infosec Twitter.
3. **Benchmark post**: *"agent-undo vs Cursor checkpoints vs nothing: a measured comparison"* — receipts and graphs.
4. **Integration tutorial**: *"Setting up agent-undo with Claude Code hooks in 30 seconds"* — short, screencast-driven.

## Anti-patterns to avoid

The research turned up failure modes:

- ❌ Telemetry on by default (kills trust)
- ❌ Forced cloud signup (kills adoption)
- ❌ "Coming soon" in pitch (kills credibility)
- ❌ Docker-required install (kills time-to-wow)
- ❌ Multi-step config file (kills first impression)
- ❌ Bury the GIF below the fold (kills the README)
- ❌ Missing benchmark numbers (kills the hook)
- ❌ Slow issue replies in week 1 (kills momentum)
- ❌ "Looking for collaborators" in launch post (looks weak)
- ❌ Mentioning competitors negatively in the README (looks petty — name the villain in the launch post, not the README)

## Success metrics

| Metric | Week 1 target | Month 1 target |
|---|---|---|
| GitHub stars | 1,000 | 5,000 |
| HN points | 200+ (front page) | — |
| Install count (telemetry-free, count via GH releases) | 5,000 | 25,000 |
| Twitter impressions | 500k | 2M |
| Issues opened | 50 | 200 |
| External blog mentions | 10 | 50 |

If week 1 stars < 300: the demo GIF wasn't sharp enough. Iterate and re-launch a "v0.2 with X" post in two weeks.

## Backup plan

If the launch underperforms (<300 stars in 48h), the pivot options:
1. **Lean harder on the security angle**: rebrand subtitle to *"File integrity monitoring for AI coding agents"*, repost on infosec Twitter and r/netsec
2. **Lean harder on the editor-specific pain**: write a post just for r/cursor — *"I built the rollback Cursor should have shipped"*
3. **Add the missing integration**: Cursor extension. Re-launch as *"Now works with Cursor"*

The core code is sound either way; the launch is just a marketing event.
