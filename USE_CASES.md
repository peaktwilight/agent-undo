# USE_CASES.md — the pretext-playbook launch plan for agent-undo

> Based on how **pretext** actually went viral — the exact tweet, the exact README opener, the exact demo approach — here's how agent-undo should launch.

---

## Part 0 — Which project is the user actually referencing?

**Confirmed: the viral tool is `chenglou/pretext`**, not `prek`. Both are real, both shipped in early 2026, and the user appears to have been conflating them. Disambiguation:

| Project | What it is | Stars | Launch | Viral? |
|---|---|---|---|---|
| **`chenglou/pretext`** | 15KB TypeScript multiline text measurement & layout library, by Cheng Lou (ex-React core, react-motion author, currently at Midjourney) | **41.2k** (≈14k in first 48h) | **March 27, 2026** on X | **Yes — 19M X views in 48h, Techmeme front page, Simon Willison writeup, Tobi Lütke amplification, PC Gamer pickup** |
| `j178/prek` | Rust drop-in rewrite of `pre-commit` (Git hook manager) | 7.2k | Late 2025 / early 2026 | Modest dev-tool traction (Home Assistant, CPython, FastAPI, Airflow adoption posts) — not a Twitter blowup |

`prek` is a respectable Ruff-style Rust rewrite that sells on benchmarks (10x faster, drop-in compatible). `pretext` is a one-day cultural event. **The launch playbook the user is intuitively chasing is pretext's**, so that's the one we reverse-engineer.

A prior draft of this file (now overwritten) was modeled on `prek`. The user has corrected that — they meant `pretext`. The two projects represent **two different launch archetypes**, and pretext's is the one that fits agent-undo's wedge:

- **prek archetype**: rewrite a beloved-but-slow incumbent in Rust, win on benchmarks, get adopted file-by-file by big projects. Slow, durable, dev-tool-press-driven.
- **pretext archetype**: ship a primitive nobody knew they needed, write a personal letter to a tribe, claim civilizational stakes, let the community build the demo gallery. Fast, explosive, mass-amplification-driven.

agent-undo is closer to pretext's shape: a **missing primitive** (rollback for AI file edits), aimed at a **wounded tribe** (anyone who's ever lost code to an agent), with a **single memorable verb** (`oops`). Use the pretext playbook.

Sources confirming all of the above are listed in the appendix.

---

## Part 1 — How pretext actually went viral

### 1.1 The launch tweet, verbatim

Posted by [@_chenglou](https://x.com/_chenglou/status/2037713766205608234), March 27, 2026:

> *"My dear front-end developers (and anyone who's interested in the future of interfaces): I have crawled through depths of hell to bring you, for the foreseeable years, one of the more important foundational pieces of UI engineering (if not in implementation then certainly at least in concept): Fast, accurate and comprehensive userland text measurement algorithm in pure TypeScript, usable for laying out entire web pages without CSS, bypassing DOM measurements and reflow"*

A continuation tweet added:

> *"The engine's tiny (few kbs), aware of browser quirks, supports all the languages you'll need, including Korean mixed with RTL Arabic and platform-specific emojis... achieved by showing Claude Code and Codex the browser's ground truth, having them measure & iterate against those at every significant container width, running over weeks."*

**What's load-bearing in this opener:**

1. **Vocative address** (*"My dear front-end developers"*) — it's a letter, not a product launch. Intimate, personal, parasocial.
2. **Suffering as legitimacy** (*"I have crawled through depths of hell"*) — the cost of caring is the proof of value.
3. **Civilizational stakes** (*"for the foreseeable years, one of the more important foundational pieces of UI engineering"*) — not "I made a thing," it's "I changed a layer of the stack." Brazenly grandiose.
4. **Concrete-and-precise spec in the same breath** — "userland text measurement algorithm in pure TypeScript, bypassing DOM measurements and reflow." You know exactly what it is by sentence two.
5. **The AI-vibe-coded provenance** (in the followup) — leans into 2026's dominant narrative instead of hiding from it.

There is **no GIF in tweet #1**. The first tweet is pure text + a link card preview of the README. The visual fireworks come from the **community demos** the next 24 hours. This is not what the current `LAUNCH.md` plan assumes, and it is the single biggest correction.

### 1.2 The README opener, verbatim

From [github.com/chenglou/pretext](https://github.com/chenglou/pretext):

> **Pretext**
>
> *"Pure JavaScript/TypeScript library for multiline text measurement & layout. Fast, accurate & supports all the languages you didn't even know about."*
>
> *"Pretext side-steps the need for DOM measurements (e.g. `getBoundingClientRect`, `offsetHeight`), which trigger layout reflow, one of the most expensive operations in the browser."*

**What's load-bearing in the README opener:**

1. **Deflated, technical, boring** — opposite tone from the tweet. The tweet is the manifesto, the README is the spec sheet. Two registers, two audiences.
2. **Names the villain in the first paragraph** — `getBoundingClientRect`, `offsetHeight`, "layout reflow," "most expensive operation in the browser." Every front-end dev has cursed these. Instant recognition.
3. **Doesn't try to sell.** No bullets, no emoji, no pricing table, no "trusted by." It says what it is and what it kills.
4. **Two use cases listed in the README** — only two, both in plain prose:
   - Measure paragraph height without DOM interaction (`prepare()` + `layout()`)
   - Manually lay out paragraph lines for canvas/SVG (`prepareWithSegments()` + `layoutWithLines()`)

That's it. **The README does not list the 12+ use cases the community went on to build.** The README ships with the floor; the community builds the ceiling. This is critical and we will copy it.

### 1.3 The demo approach (no single GIF — and why that worked)

Pretext did **not** ship with one canonical demo GIF. It shipped with a **demo gallery** at `chenglou.me/pretext/` containing four officially-built demos:

> **Accordion. Bubbles. Dynamic layout. ASCII art.**

The "ASCII art" demo became the icon — **Bad Apple!! rendered as real-time ASCII text animation** running through the layout engine. That's the screenshot you saw on Twitter for 48 hours straight. Within 24 hours the community had added:

- Pretext Breaker (Breakout with text-block bricks)
- Tetris × Pretext
- Star Wars opening crawl
- Drag-Sprite Reflow (paragraph reflowing around a draggable sprite in real time)
- Face × Pretext (TensorFlow.js face tracking driving typography)
- Illustrated Manuscript (medieval scroll with animated dragon)
- Explosive text (shatter particles)
- Responsive Testimonials

The demo strategy was: **ship 4 toys yourself, make the API trivial enough that the timeline ships 12 more by tomorrow morning.** The viral surface area is community-built, not creator-built.

### 1.4 The villain framing

Pretext did **not** name a competitor company as villain. It named a *primitive* — `getBoundingClientRect` — and the *physical phenomenon* it triggers (layout reflow). The villain is a 30-year-old browser bottleneck, not a vendor. That's a much more comfortable enemy: nobody defends `getBoundingClientRect`, and you don't pick a fight with Google or Mozilla.

### 1.5 Amplification — who reshared

Within 48 hours, confirmed amplifiers:

- **Simon Willison** — [full writeup on simonwillison.net](https://simonwillison.net/2026/Mar/29/pretext/) (Mar 29) → **Techmeme front page** (Mar 30, p4)
- **Tobi Lütke** (Shopify CEO) — quoted/replied; mentioned in trade-press reaction roundups
- **VentureBeat** — full article ("Midjourney engineer debuts new vibe-coded open-source standard")
- **PC Gamer** — picked up the human-interest "crawled through depths of hell" angle (this is the moment a dev tool jumps the audience fence and becomes a *story*, not a *launch*)
- **Dataconomy, Cloudmagazin, TechBriefly** — trade press cascade
- **36kr (China)** — international pickup, surfaced the Bad Apple demo to a second audience
- **GenAI.Works** ([@GenAI_Now](https://x.com/GenAI_Now/status/2038137546832847194)) — AI-newsletter QT that re-amplified to the AI crowd
- **Cheng Lou's own follow-up** the next morning ([this tweet](https://x.com/_chenglou/status/2037964564072210899)): *"Jeeesus, I wake up and it's like my timeline claimed Pretext cured cancer or something lol. Thanks for the sliiightly hyperbolic affection folks. Anyway, here's the standard rich text demo!"* — this is a master-class second tweet: humble, posts another demo, keeps the wave going for a second day.

**Trajectory:** 0 → 14k stars in 48h → 19M X impressions → 41.2k stars at the time of writing (~10 days later). Still adding ~2k stars/week.

### 1.6 Number of use cases pretext officially listed

**Two.** That's it. The README enumerates exactly two API use cases. The community generated everything else.

---

## Part 2 — The pretext playbook, applied to agent-undo

### 2.1 The launch tweet (the agent-undo equivalent)

Pretext won by writing a *letter* to a tribe, claiming *civilizational stakes*, and shipping a *boring spec* in the README. We do the same, with our tribe (anyone who's lost code to an AI agent) and our verb (`oops`).

**Draft launch tweet (tweet 1 of thread):**

> *My dear AI-coding survivors, my Cursor refugees, my Claude-Code night-shifters: I have spent four weeks watching agents nuke uncommitted code so I could give you the missing layer none of the editors shipped — a 5MB Rust binary that snapshots every file your agent touches and rolls back any session with one command. Editor-agnostic. Local-first. Zero config. The verb is `oops`.*
>
> agent-undo.com · github.com/peak/agent-undo

Note the deliberate copies from pretext:

- **Vocative address** (*"My dear AI-coding survivors"*) — letter, not pitch.
- **Suffering as legitimacy** (*"four weeks watching agents nuke uncommitted code"*) — and ours is *true*, which is even better.
- **Civilizational framing** (*"the missing layer none of the editors shipped"*) — we are not "another tool," we are a layer the platform vendors failed to ship.
- **Boring concrete spec in the same breath** — *"5MB Rust binary, snapshots every file write, rolls back any session with one command."* You know what it is by the second sentence.
- **A single memorable verb** — *"The verb is `oops`."* Pretext didn't have one of these. We do. Use it.

### 2.2 The README opener (deliberately deflated)

Pretext's README does not match the energy of its tweet. Ours shouldn't either.

**Draft README opening, post-launch version:**

> # agent-undo
>
> *Local-first rollback for AI coding agents. A single 5MB binary that snapshots every file your agent writes and lets you undo any session with one command.*
>
> *agent-undo side-steps the need for editor checkpoints, IDE history, or after-the-fact `git reflog` recovery, all of which silently fail when the agent has been given write access to the filesystem and acted faster than your save loop.*

This mirrors pretext's two-paragraph structure:

1. What it is, in plain English, no adjectives.
2. What it kills, named explicitly. Our `getBoundingClientRect` is *editor checkpoints, IDE history, and `git reflog`.*

No emoji. No "trusted by." No bullets above the fold. Quiet authority, then the demo, then the spec.

### 2.3 The demo strategy — gallery, not GIF

This is the biggest single departure from `LAUNCH.md`'s current plan, and it's the one that matters most.

`LAUNCH.md` currently says: *"Spend a full day making the 15-second demo GIF perfect. This GIF is the launch."* That is the **Aider/Ollama** playbook. Pretext used a different playbook and got 10x the result.

**The pretext playbook is: ship 4 demos in a gallery, make the API so legible the community builds 12 more in 24 hours.**

For agent-undo, the equivalent gallery — call it `agent-undo.com/demos` — should be **four short screencaps, each ≤8 seconds**, each demonstrating a different rescue:

| # | Demo name | What it shows | Why it's shareable |
|---|---|---|---|
| 1 | **`oops` after a 5-file Claude Code rampage** | Claude wrecks `auth.rs`, `middleware.rs`, `lib.rs`, `tests/`, `Cargo.toml`. User types `agent-undo oops`, hits Y, files restored, tests green. | The hero shot. Ships in tweet #2 and the README first viewport. |
| 2 | **Cursor deletes itself, agent-undo brings it back** | Reproduce the literal Cursor "Agent Review Tab" bug from the forum (deanrie's quote). agent-undo restores. | Direct receipt-vs-fix confrontation. Becomes the QT bait. |
| 3 | **`agent-undo blame auth.rs`** | A file with mixed human + agent edits. `blame` shows per-line attribution: "claude-code 14:32," "you 09:15," "cursor 11:40." | The screenshot devs will tweet *without watching the GIF* — pure visual hook. |
| 4 | **`agent-undo tui` timeline scrubber** | Arrow-key through every state of a file, watching the diff redraw. The interactive feel is the wow. | Becomes the "wait, you can do this?" reply in threads. |

Then on launch day, prompt people in tweet #5 of the thread: *"What rescue do you want to see? PR a demo to /demos and I'll ship it."* That is the pretext community-demo flywheel, lifted directly.

**The agent-undo equivalent of "Bad Apple"** — the iconic, slightly absurd showcase clip — is a 30-second screen recording where someone literally lets Claude Code run with `--dangerously-skip-permissions` for 60 seconds in a real codebase, and `agent-undo` rolls all of it back to t=0. Call it **`oops apocalypse`**. Build it. It is the meme.

### 2.4 The villain — name a primitive, not a vendor

Pretext won by naming `getBoundingClientRect`, not Chrome. We should name **"the agent's filesystem write,"** not Cursor.

`VILLAIN.md` currently leans hard on the Cursor staff "close the Agent Review Tab" quote, and that quote is genuinely incredible — but it should live in the **launch thread** (tweet #3), not the README. The README villain paragraph should name the underlying primitive:

> *Every AI coding agent today writes to your filesystem the same way you would: directly, immediately, and irreversibly. Editor checkpoints are an in-memory afterthought. `git` is not a save loop. When the agent moves faster than your commit cadence, the only safety net is an out-of-band log of every byte that hit disk. agent-undo is that log.*

Then in the **thread**, you cash in the Cursor receipts. PHILOSOPHY.md/VILLAIN.md already has the editorial discipline right ("don't be petty in the README itself"). This confirms it.

### 2.5 The use case list — keep it small in the README

Pretext listed **two** use cases in its README. The community wrote 12+. We do the same.

**README use cases (the official list — keep it to four):**

1. **Recover from a bad agent edit.** The hero use case. `agent-undo oops`.
2. **Audit what an agent actually changed.** `agent-undo log --agent claude-code --since 1h` and `agent-undo diff --session <id>`.
3. **Per-line agent attribution.** `agent-undo blame <file>` — like `git blame`, but tells you which agent (or human) wrote each line.
4. **Pin a known-good state before letting an agent loose.** `agent-undo pin "before refactor"` then later `agent-undo restore --pin "before refactor"`.

That's it in the README. Resist the urge to list more.

**The "extended" use cases (for the launch thread, blog posts, and the community-demo gallery — not the README):**

5. Recover after an `rm -rf` agent typo
6. Recover after a `--dangerously-skip-permissions` rampage
7. Recover deleted test files (the most common Cursor failure mode per VILLAIN.md)
8. Recover when `.git` itself was nuked (agent-undo's CAS is independent of git)
9. Find which agent introduced a regression three days ago
10. Diff two agent sessions side-by-side to compare two refactors
11. Roll back just *one file* from a multi-file session
12. Restore the state from "before lunch" without knowing the commit hash
13. Bisect a bug across agent sessions instead of git commits
14. Snapshot before letting Codex run a long unattended task
15. Pre-flight a destructive command with `agent-undo exec -- <cmd>`
16. CI integration: fail the build if any uncommitted state is missing from agent-undo
17. Forensics: "show me every file Claude touched yesterday between 2pm and 3pm"
18. Recover from a merge conflict introduced by an agent that mis-resolved one
19. Compare what the agent *said* it did vs. what it *actually* wrote (the muzani "rewrite the mock to pass" case from VILLAIN.md)
20. The "before-bed safety net" — pin nightly, sleep easy

**Exactly 20.** The README ships with 4. The blog post enumerates all 20. The community will surface use cases 21-50 in issues within a week.

### 2.6 Personas (so the use cases above land for the right people)

Pretext spoke to a single tribe ("front-end developers"). agent-undo's tribe is broader, so we lean on three personas — but in the launch thread, we open with **only one** (the Cursor refugee). The others come in week 2 content.

| # | Persona | What they lost | What they say in their head | The use case that converts them |
|---|---|---|---|---|
| **A** | **The Cursor Refugee** | A weekend's work to a checkpoint that didn't restore | *"I literally cannot trust the undo button anymore"* | #1 (`oops`), #7 (deleted tests), #4 (pin) |
| **B** | **The Claude Code Power User** | A test file Claude "consolidated away" while it was running unattended overnight | *"I want process-level forensics, not chat scrollback"* | #3 (`blame`), #9 (which agent introduced this), #17 (forensics window) |
| **C** | **The Vibe-Coding Founder** | Four months of work over multiple incidents (the $5,500 / 4-month story from VILLAIN.md) | *"I need a safety net that survives me hitting accept-all at 2am"* | #2 (`--dangerously-skip-permissions` rampage), #20 (before-bed pin), #14 (pre-flight long task) |

**Persona A is the launch.** Personas B and C are the second and third week's blog posts. Don't try to address all three on day one.

### 2.7 The 15-second hero GIF storyboard

Pretext didn't ship a hero GIF and that worked for *Cheng Lou*. agent-undo is in a different category — we are a CLI tool, not a layout primitive — and the empirical evidence (Aider, Ollama, Bun, uv, ripgrep) is that **CLI launches need a hero GIF**. So we keep the GIF, but as **demo #1 of the gallery**, not as the only artifact.

**Storyboard, 15 seconds, 24fps, target ≤1.8MB, autoplay-on-loop, captioned (no audio dependency):**

| t | Frame | What happens | Caption overlay |
|---|---|---|---|
| 0.0–1.5s | Split terminal: Claude Code (left), `auth.rs` open in `bat` (right). Pristine. Tests passing on the bottom strip. | Establishing shot. Quiet. | *"You. Your code. A normal Tuesday."* |
| 1.5–3.5s | User types into Claude Code: *"Refactor src/auth.rs to use the new token format."* Claude responds, edits begin streaming. | Edits flying across 5 files. | *"You ask for a refactor."* |
| 3.5–6.0s | Right pane updates. `auth.rs` shrinks visibly: -87 lines, +12. `tests/auth.rs` flashes "DELETED." Test strip turns red. | The horror. Hold for one extra beat. | *"You get a deletion."* |
| 6.0–7.0s | User types: `agent-undo oops` | The reveal of the verb. | (no caption — let the command speak) |
| 7.0–10.0s | The `oops` prompt renders, exactly as in `README.md`: *"Last agent action: claude-code, session 14:32-14:34, edited 5 files... Roll back this entire session? [Y/n]"* | Hold long enough to read. This is the screenshot people will tweet. | *"One command. One keystroke."* |
| 10.0–11.0s | User hits `y`. | Crisp keystroke (silent fallback). | — |
| 11.0–13.5s | All five files snap back. Test strip flashes green. `auth.rs` content restored verbatim. | The exhale. | *"Restored. All five files."* |
| 13.5–15.0s | End card: black background, two lines: **agent-undo** / *Ctrl-Z for AI coding agents.* `agent-undo.com` underneath. | Fade. | — |

**Production notes:**
- Record at 2x size (retina), downscale for the GIF — readable text in the prompt is non-negotiable.
- Use `vhs` (charm.sh) so the demo is reproducible from a `.tape` file checked into the repo. This *itself* becomes a small social moment — devs love when the README's GIF is built from a script in the repo.
- Dark + high-contrast terminal theme. JetBrains Mono. No fancy fonts the embed renderer might fail on.
- Caption text in a quiet sans, lower-third, 60% opacity. Never block the prompt.
- The hold on the `oops` prompt is the most important 3 seconds of the launch. Do not rush it.

### 2.8 The Twitter thread (10 tweets, pretext-shaped)

Pretext's actual thread was short (3 tweets + a follow-up the next day). agent-undo's category demands more context, but we keep tweets 1, 2, and the closer disciplined.

**Tweet 1 — The letter (no media, just the link card):**

> My dear AI-coding survivors, my Cursor refugees, my Claude-Code night-shifters:
>
> I have spent four weeks watching agents nuke uncommitted code so I could give you the missing layer none of the editors shipped — a 5MB Rust binary that snapshots every file your agent touches and rolls back any session with one command.
>
> Editor-agnostic. Local-first. Zero config. The verb is `oops`.
>
> agent-undo.com

**Tweet 2 — The hero GIF.** No text, or one line: *"This is the whole pitch."*

**Tweet 3 — The villain receipts.** Screenshot of deanrie's "close the Agent Review Tab" quote from the Cursor forum. Caption:
> Cursor's official fix for losing your work in March 2026 was *"close the Agent Review Tab before the agent makes edits."* That is on the record. Their staff said it. I built the alternative.

**Tweet 4 — "It's not just Cursor."** Screenshot of Cline issue #5124 title. Caption:
> Cline issue #5124, July 2025: *"Cline autonomously delete files without keeping track of the deleted/changed files. Very Dangerous and Critical Issue!!!"*
> This is an editor-class problem, not a Cursor problem. agent-undo is editor-agnostic on purpose.

**Tweet 5 — How attribution works (the technically interesting beat).**
> agent-undo watches every file write and labels it with which process did it: claude-code, cursor, cline, aider, codex, or you. Three layers: kqueue/inotify, /proc PID walk, hook shims. ~600 lines of Rust. Blog post next week.

**Tweet 6 — The install line.**
> ```
> curl -fsSL https://agent-undo.com/install.sh | sh
> agent-undo init
> ```
> 5MB binary. No daemon you didn't start. No telemetry. No account. No cloud.

**Tweet 7 — Use case spread.** A 4-up image: `oops`, `blame`, `log`, `tui` screenshots. Caption:
> Four things you can do in the first minute:
> 1. `agent-undo oops` — undo the last agent action
> 2. `agent-undo blame auth.rs` — per-line agent attribution
> 3. `agent-undo log --agent claude-code --since 1h` — what did Claude touch?
> 4. `agent-undo tui` — interactive timeline scrubber

**Tweet 8 — Tag the world.**
> Built because @cursor_ai @AnthropicAI @cline @paulgauthier @OpenAIDevs all built agents that write to disk and none of them built a real undo. Love your tools. Hate losing code. Here's the safety net.

**Tweet 9 — The community-demo flywheel (the pretext move).**
> What rescue do you want to see demoed? Reply with the worst thing your agent ever did to you and I'll record `agent-undo oops` undoing it. Best ones go in the README gallery.

**Tweet 10 — The closer.**
> If you've ever lost code to an AI agent, this is the safety net I wish I'd had. RT if you've been there. Build with peace of mind.
>
> github.com/peak/agent-undo · agent-undo.com

**Day-2 follow-up tweet (the Cheng Lou move):**
> Woke up to the timeline acting like agent-undo cured cancer. Thanks for the wildly hyperbolic affection folks. Here's the demo I should have shipped yesterday: `oops apocalypse` — Claude with `--dangerously-skip-permissions` running 60s in a real repo, then one keystroke rolls all of it back. [video]

This is the pretext follow-up tweet, almost beat-for-beat. It works because it's a humble victory lap *and* it ships another piece of content, which keeps the wave alive for a second day.

### 2.9 README structure recommendation

Read top-to-bottom in one minute. Modeled on pretext's README rhythm.

```
# agent-undo

[15-second hero GIF — autoplay, loop, no audio]

> Local-first rollback for AI coding agents. A single 5MB binary that
> snapshots every file your agent writes and lets you undo any session
> with one command.

agent-undo side-steps the need for editor checkpoints, IDE history, or
after-the-fact `git reflog` recovery — all of which silently fail when
the agent has been given write access to the filesystem and acted faster
than your save loop.

## Install

curl -fsSL https://agent-undo.com/install.sh | sh
agent-undo init

## The killer command

[the `oops` prompt block from current README, verbatim]

## What it does

1. Recover from a bad agent edit — `agent-undo oops`
2. Audit what an agent actually changed — `agent-undo log` / `diff`
3. Per-line agent attribution — `agent-undo blame <file>`
4. Pin a known-good state — `agent-undo pin <label>`

(That's it. Four use cases in the README. The rest live in the demo
 gallery and the blog posts.)

## Demos

→ agent-undo.com/demos — gallery of rescue scenarios.
   PR your own to /demos and we'll ship it.

## How attribution works

[2-paragraph explanation, link to ARCHITECTURE.md]

## CLI surface

[the existing CLI table from README.md, unchanged]

## Why this exists

[3-sentence neutral problem statement, link to LAUNCH.md / VILLAIN.md
 for the receipts. Don't be petty in the README itself.]

## Project docs

[links to ARCHITECTURE.md, PHILOSOPHY.md, RESEARCH.md, LAUNCH.md, VILLAIN.md]

## License

MIT
```

**What's missing from the current `README.md` that this fixes:**

- The current README puts the killer command halfway down. **The GIF and the killer command should be the first two things on the page, in that order.** Pretext put the demo gallery link in the first viewport. We should put the GIF *literally* in the first viewport.
- The current README has a long competitive landscape ("Why this isn't a duplicate") above the fold. **Cut it.** Move it into RESEARCH.md (where it already lives anyway). Pretext does not list competitors in the README. Quiet authority.
- The current README has 17 CLI commands listed before any use cases. **Flip the order.** Use cases first (4 of them), then the install, then the full CLI table for the people who want it.

---

## Part 3 — The single sentence that unlocks all of this

Pretext won because Cheng Lou wrote the launch tweet as a *letter to a tribe*, claimed *civilizational stakes for a primitive*, and then shipped a *boring spec sheet* in the README — and then let the community build the demo gallery.

agent-undo's tribe is wider and angrier than pretext's, the villain receipts are stronger, and the verb (`oops`) is more memorable than anything pretext has. The playbook fits us better than it fit pretext.

**Do exactly what Cheng Lou did. Same tweet shape. Same README rhythm. Same demo-gallery flywheel. Same humble day-2 follow-up.** That is the launch.

---

## Appendix — Sources

Primary (the pretext launch artifacts):
- [Cheng Lou launch tweet (verbatim, full text quoted in §1.1)](https://x.com/_chenglou/status/2037713766205608234)
- [Cheng Lou day-2 follow-up tweet ("cured cancer")](https://x.com/_chenglou/status/2037964564072210899)
- [chenglou/pretext on GitHub (41.2k stars at writing)](https://github.com/chenglou/pretext)
- [Pretext demo gallery](https://chenglou.me/pretext/)
- [awesome-pretext community demo index](https://github.com/bluedusk/awesome-pretext)

Coverage / amplification trail:
- [Simon Willison: Pretext (Mar 29)](https://simonwillison.net/2026/Mar/29/pretext/)
- [Techmeme front page (Mar 30)](https://www.techmeme.com/260330/p4)
- [VentureBeat: Midjourney engineer debuts Pretext](https://venturebeat.com/technology/midjourney-engineer-debuts-new-vibe-coded-open-source-standard-pretext-to)
- [PC Gamer: "I have crawled through depths of hell"](https://www.pcgamer.com/software/browsers/i-have-crawled-through-depths-of-hell-one-coders-suffering-is-a-potential-joy-to-every-web-user-as-their-project-could-make-sluggish-browsers-a-thing-of-the-past/)
- [Dataconomy](https://dataconomy.com/2026/03/31/new-typescript-library-pretext-tackles-text-reflow-bottlenecks/)
- [36kr coverage (China)](https://eu.36kr.com/en/p/3745083757068551)
- [GenAI.Works QT amplification](https://x.com/GenAI_Now/status/2038137546832847194)

Disambiguation (the *other* project the user may have been thinking of):
- [j178/prek (Rust pre-commit rewrite — 7.2k stars, different project)](https://github.com/j178/prek)
- [Home Assistant: Replacing pre-commit with prek (Jan 2026)](https://developers.home-assistant.io/blog/2026/01/13/replace-pre-commit-with-prek/)
