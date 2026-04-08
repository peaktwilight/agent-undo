# Research — competitive landscape & naming

This doc captures the research findings as of April 2026 that justify both the project direction and the name choice.

## Verdict

**Wide open.** ~15-20 weekend projects in this space, none above 25 stars, most abandoned or days old. No Show HN hit. The editor built-ins users depend on are *publicly broken* with Reddit receipts. The Rust + single-binary + multi-agent attribution + memorable-verb combination is **not occupied**.

Biggest risk is **not competition** — it's that Cursor or Anthropic fix their built-ins and absorb the market. Ship in weeks, not months.

## Direct competitors (OSS)

| Project | Stars | Lang | Status | Notes |
|---|---|---|---|---|
| `holasoymalva/claude-code-rewind` | 23 | Python | Stale 7 mo | "Time machine for Claude Code." Single-agent, abandoned. |
| `omeedcs/vibetracer` | 20 | **Rust** | Active (2 wk old) | Closest in language + scope. Framed as "tracer," not rollback. No traction. |
| `tomsun28/agentshield` | 13 | TS | Jan 2026 | "Undo button for AI messing up your local computer." Whole-computer scope, not project-scoped. Show HN got 1 point. |
| `0xdismissals/rewind` | 1 | Go | Jan 2026 | **Name collision risk for "rewind"**. "Local-first snapshot and rollback for AI code edits, no git." Identical pitch. Zero traction. |
| `Vvkmnn/claude-vigil-mcp` | 8 | TS | — | MCP server for Claude Code recovery. Single-agent. |
| `yesonsys03-web/VibeLign` | 4 | Python | Active | "Checkpoints, undo, anchors, MCP, secret protection for Claude Code, Cursor..." Closest to multi-agent goal. Tiny. |
| `mohshomis/ckpt` | 4 | TS | 4 days old | "Automatic checkpoints for AI coding sessions on top of git." |
| `moltenlabs/hutch` | 0 | **Rust** | Dec 2025 | "Checkpoint and undo system for AI agent sessions." No traction. |
| `A386official/diffback`, `HadiFrt20/snaprevert`, `syi0808/complete-checkpoint`, `AEsho12/Gradus`, `metacogma/agentgram` | 0–1 each | mixed | 2025–2026 | All weekend experiments. None have traction. |

**Pattern:** ~20 projects, almost all <10 stars, almost all abandoned or days old. Many are Python or TypeScript. None combine Rust + single-binary + multi-agent attribution + a memorable panic-button UX.

## Editor built-ins (the real competition)

| Editor | Built-in | Status |
|---|---|---|
| **Cursor** | Checkpoints | **Demonstrably broken.** r/cursor: *"Revert and Undo are now dangerously broken, burn model calls, and don't actually match the state you're trying to revert/undo to"* (9 pts, Oct 2025). *"Is the Restore Checkpoint button broken?"* (4 pts). *"SYMLINKS ARE BROKEN — Cursor sabotage and destroy all my work 10 times per day."* |
| **Claude Code** | None first-class | Users scrape `~/.claude` session logs to recover. Active community pain point. r/ClaudeAI: *"Claude Code's conversation logs are a recovery goldmine."* |
| **Aider** | Auto-commit to git | Solved for aider, only aider. |
| **Cline** | Workspace checkpoints | VSCode-bound, works only inside Cline. |
| **Continue.dev** | Edit history | In-memory, session-scoped, not a rollback store. |

The editors are trying to solve this and failing. That's the gap.

## Why agent-undo wins

1. **Single Rust binary** — only `vibetracer` (tracer-framed) and `hutch` (0 stars) are Rust. Neither is positioned as the rewind/oops button.
2. **Editor-agnostic multi-agent attribution** — `VibeLign` attempts this at 4 stars. Everyone else is single-agent. The multi-agent attribution layer is the genuinely unfilled niche.
3. **FS-watch + CAS, not git-based, not MCP-only** — most competitors wrap git or run as MCP servers. A pure fs-watch CAS approach survives when agents nuke `.git` itself.
4. **One memorable command (`oops`)** — nobody has nailed the panic-button UX. "agent-undo oops" is the most demoable verb in the space.
5. **Founder credibility on safety/forensics** — cybersecurity background (PhishMind, CVEs) makes "treat your agent as an untrusted process" a credible identity.

## Naming research

### Candidates evaluated

| Name | crates.io | npm | GitHub repo | Major collisions | Verdict |
|---|---|---|---|---|---|
| **rewind** | TAKEN | TAKEN | many | **rewind.ai (Limitless)**, rewind.com (SaaS backups), rewind.sh (screenpipe), rewind-ui.dev | ❌ search-hostile dead zone |
| **agent-undo** | FREE | FREE | FREE | None | ✅ clean everywhere |
| **unedit** | FREE | FREE | minor | userscript only | ✅ runner-up |
| **agentfs** | TAKEN | FREE | — | filesystem semantics misleading | ❌ |
| **redo** | TAKEN | TAKEN | — | djb's `redo` build system iconic | ❌ |
| **kintsugi** | FREE | TAKEN | — | Lightricks/Kintsugi (Xcode merge tool) | ❌ |
| **timefold** | FREE | FREE | — | TimefoldAI/timefold-solver major OSS | ❌ |
| **revert** | TAKEN | TAKEN | — | generic git term | ❌ |
| **anchor** | TAKEN | TAKEN | — | **Solana's Anchor framework** | ❌ |
| **chronicle** | TAKEN | TAKEN | — | **Google Chronicle (security SIEM)** | ❌ direct collision with security framing |
| **trace** | TAKEN | TAKEN | — | observability term, unsearchable | ❌ |
| **rollback** | TAKEN | TAKEN | — | DB term, unsearchable | ❌ |
| **scrub** | TAKEN | TAKEN | — | data destruction connotation | ❌ |
| **ctrlz / ctrl-z** | mixed | mixed | meme | weak search | ❌ |
| **keepsafe** | FREE | FREE | — | Keepsafe consumer photo vault | ⚠ |

### Decision: **agent-undo**

**Reasoning:**

1. **Free everywhere** — GitHub, crates.io, npm, agent-undo.com all available
2. **Search-friendly** — "agent undo github" surfaces it instantly with no noise
3. **Descriptive** — boring-and-descriptive is a feature for OSS dev tools (see ripgrep, fd, just, watchexec)
4. **Pronounceable** — two syllables, no ambiguity
5. **Zero brand collision** — confirmed across all major package registries and TLDs
6. **Cyber-adjacent vibe** — "undo = safety net" pairs with the security framing
7. **Future-proof** — works for v1 single-tool focus AND v2 plugin ecosystem (`agent-undo-slack-notify`, `agent-undo-cloud-backup`, etc.)

**Why not `rewind`:**
- rewind.ai is a well-known Mac AI memory tool (now Limitless) — direct AI-space collision
- rewind.com is a SaaS backups company — direct backups-space collision
- rewind.sh is the screenpipe project — direct dev-tool collision
- Search SEO is a permanent dead zone
- 0xdismissals/rewind already exists on GitHub with the same pitch

**Why not `unedit`:** strong runner-up, available everywhere, but slightly less clear what it does on first read. `agent-undo` says exactly what it does.

## The window

This space won't stay open forever. Three closing forces:

1. **Cursor or Anthropic fix their built-ins** (most likely threat)
2. **A funded startup ships this exact thing** (less likely — no signal yet)
3. **One of the existing weekend projects breaks out** (vibetracer is the closest at 20 stars; manageable)

**Estimated window: 2–4 months.** Ship v1 in 4 weeks. After launch, the incumbent advantage of being first-to-viral compounds rapidly.

## Sources

- [vibetracer](https://github.com/omeedcs/vibetracer)
- [claude-code-rewind](https://github.com/holasoymalva/claude-code-rewind)
- [agentshield](https://github.com/tomsun28/agentshield)
- [0xdismissals/rewind](https://github.com/0xdismissals/rewind)
- [VibeLign](https://github.com/yesonsys03-web/VibeLign)
- [Cursor forum: Revert and Undo broken](https://forum.cursor.com/t/revert-and-undo-are-now-dangerously-broken/...)
- [Rewind.ai (Limitless)](https://rewind.ai/)
- [TimefoldAI](https://github.com/TimefoldAI/timefold-solver)
- [Solana Anchor](https://github.com/solana-foundation/anchor)
- [Lightricks Kintsugi](https://github.com/Lightricks/Kintsugi)
