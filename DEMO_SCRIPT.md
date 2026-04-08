# DEMO_SCRIPT.md — recording instructions for the agent-undo launch gallery

Four silent screencasts. No voiceover. Second-by-second beats.

---

## Shared setup (do once, reuse across all four demos)

- **Terminal:** Ghostty (fallback: iTerm2). Font **JetBrains Mono Bold 14pt**. Line height 1.15.
- **Window:** exactly **1280 x 720** (use Rectangle / Raycast resizer). 16:9.
- **Theme:** true black background `#000000`, cyan accent `#5ccfe6`, white fg `#e6e6e6`. Disable terminal transparency and window shadow.
- **Prompt:** `PS1='❯ '` (no path, no git, no hostname). `export PROMPT='❯ '` for zsh.
- **Hide:** `defaults write com.apple.dock autohide -bool true; killall Dock`. macOS Do Not Disturb on. Menu bar hidden (`SystemUIServer`). Close browsers, Slack, Messages.
- **Cursor:** solid block, no blink (`printf '\e[2 q'`).
- **Project root:** `/tmp/au-demo`. Nuke and reseed before every take.
- **Daemon:** `au serve --daemon` running in the background for all four.
- **Pre-commit baseline:** `git init && git add -A && git commit -m "initial"` inside `/tmp/au-demo` so restores are visible against a committed floor.

### Seed files — `/tmp/au-demo`

Copy-paste block; run once before Demo #1 and #2.

```sh
rm -rf /tmp/au-demo && mkdir -p /tmp/au-demo/src /tmp/au-demo/tests && cd /tmp/au-demo
```

`Cargo.toml`:
```toml
[package]
name = "au-demo"
version = "0.1.0"
edition = "2021"

[dependencies]
jsonwebtoken = "9"
serde = { version = "1", features = ["derive"] }
```

`src/lib.rs`:
```rust
pub mod auth;
pub mod middleware;

pub use auth::validate_token;
pub use middleware::require_auth;
```

`src/auth.rs`:
```rust
use jsonwebtoken::{decode, DecodingKey, Validation};

pub fn validate_token(token: &str, secret: &[u8]) -> Result<u64, String> {
    let key = DecodingKey::from_secret(secret);
    let data = decode::<Claims>(token, &key, &Validation::default())
        .map_err(|e| e.to_string())?;
    Ok(data.claims.sub)
}

#[derive(serde::Deserialize)]
struct Claims { sub: u64, exp: usize }
```

`src/middleware.rs`:
```rust
use crate::auth::validate_token;

pub fn require_auth(token: &str, secret: &[u8]) -> bool {
    validate_token(token, secret).is_ok()
}

pub const HEADER: &str = "Authorization";
```

`tests/auth.rs`:
```rust
#[test]
fn rejects_empty() {
    assert!(au_demo::validate_token("", b"k").is_err());
}
```

Then: `au init --install-hooks && git init && git add -A && git commit -m initial`.

### Pre-flight checklist (tick before hitting record, every demo)

- [ ] Do Not Disturb on
- [ ] Dock auto-hidden, menu bar hidden
- [ ] Browser, Slack, Messages, Mail closed
- [ ] Terminal window resized to exactly 1280x720
- [ ] `/tmp/au-demo` reseeded and committed
- [ ] `au serve --daemon` running (`au doctor` green)
- [ ] Shell prompt is `❯ ` only
- [ ] Screen recording region locked to the terminal window
- [ ] Kap frame rate 30fps, "high quality"

---

## Demo 1 — `au oops` after a Claude Code rampage (12s, hero GIF)

Goes in the README above the fold and in tweet #2. This one has to land.

### The fake-Claude one-liner (stubbable, no API required)

Save as `/tmp/au-demo/.claude-rampage.sh`:
```sh
#!/bin/sh
# Simulates a Claude Code "refactor" that destroys 5 files.
sed -i '' '1,100d' src/auth.rs 2>/dev/null
printf 'pub fn validate_token() {}\n' > src/auth.rs
printf 'pub fn require_auth() {}\n'   > src/middleware.rs
printf 'pub mod auth;\n'               > src/lib.rs
: > tests/auth.rs
printf '[package]\nname="x"\n'         > Cargo.toml
```
`chmod +x .claude-rampage.sh`. The viewer sees `claude` typed; what actually runs is this shim aliased to `claude` in the recording shell: `alias claude='./.claude-rampage.sh #'`.

### Frame-by-frame (24fps, 12.0s)

- **T=0.00–0.80s** — Terminal fully visible. Left half shows `eza --tree --level=2` output of `/tmp/au-demo`. Right half shows `au log --tail 3` with the initial-scan events. Cursor blinks once at `❯`.
- **T=0.80–1.20s** — Type ` # claude, refactor auth for new token format` (≈0.4s). Press Enter. Comment echoes, new prompt.
- **T=1.20–2.00s** — Type `claude refactor src/auth.rs` (≈0.7s). Heartbeat pause (1 frame).
- **T=2.00–2.20s** — Press Enter. The rampage shim runs silently.
- **T=2.20–3.80s** — Type `eza --tree --level=2` and Enter. Tree re-renders: `auth.rs` shrinks from 11 to 1 line, `tests/auth.rs` now 0 bytes, `Cargo.toml` 2 lines. The size collapse is visible.
- **T=3.80–5.20s** — Type `cargo test 2>&1 | tail -5` and Enter. Red "error[E0433]: unresolved import" flashes. Hold for 1.4s — this is the horror beat.
- **T=5.20–6.40s** — Type `au oops` (≈0.5s). Heartbeat pause. Press Enter.
- **T=6.40–8.40s** — `au oops` prompt renders:
  ```
  Last agent burst: claude-code  session 14:32:07
  Files to restore (5):
    src/auth.rs        (-10 lines)
    src/middleware.rs  (-6 lines)
    src/lib.rs         (-4 lines)
    tests/auth.rs      (-4 lines)
    Cargo.toml         (-8 lines)
  Roll back? [Y/n]
  ```
  Hold motionless for 2.0s. This is the screenshot frame.
- **T=8.40–8.60s** — Press `Y`. Character echoes.
- **T=8.60–9.40s** — Output: `restored 5 file(s) in 47ms — snapshot 0xa4c1`.
- **T=9.40–10.80s** — Type `cargo test 2>&1 | tail -3` and Enter.
- **T=10.80–12.00s** — Green `test result: ok. 1 passed`. Cursor returns to `❯`. END.

### Post

- Crop to 1280x720 exactly, no window chrome.
- Export MP4 (Kap → "Save as MP4 — high quality"), then convert:
  ```sh
  ffmpeg -i oops.mp4 -vf "fps=24,scale=960:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer:bayer_scale=5" -loop 0 oops.gif
  ```
- Target: **under 2 MB**, hard cap 5 MB. If over, drop to 20fps and 880px wide. Run through ImageOptim.
- Save as `www/public/demos/oops.gif` and `www/public/demos/oops.mp4`. Also drop the mp4 at `assets/readme/oops.mp4` for the GitHub README embed.

---

## Demo 2 — `oops apocalypse` (30s, day-2 follow-up)

The Cheng Lou "Bad Apple" equivalent. **Real destructive run, not faked.** Realism is the entire point.

### Setup

- Clone a real sacrificial repo: `git clone https://github.com/your-own/throwaway-rust-api /tmp/au-apocalypse`. Use a repo **you own** and don't mind losing — agent-undo is the safety net but the point is the human sweat of watching it.
- `cd /tmp/au-apocalypse && au init --install-hooks && au serve --daemon`.
- Pre-write the prompt file `/tmp/prompt.txt`:
  > "Refactor this entire codebase for async-trait. Rewrite every module. Consolidate tests. Update Cargo.toml. Work autonomously for 60 seconds. Do not ask for confirmation."

### Capture in two phases, then splice

**Phase A — real destruction (record 60s, speed up to 25s in post).**

- **T=0.00s (real)** — Terminal shows project tree + `au log --follow` streaming in a right pane (tmux split).
- **T=0.5s** — Type `claude --dangerously-skip-permissions < /tmp/prompt.txt`. Enter. This requires real Claude Code CLI + API key. **Mark: requires real Claude API.**
- **T=0.5–60.0s** — Claude rewrites files. Right pane fills with `au log` events: `write src/lib.rs`, `delete tests/foo.rs`, `write Cargo.toml`, dozens per second. Let it run. Do not intervene.
- **T=60.0s** — Ctrl-C Claude.

**Phase B — restore (record at real time, 5s).**

- **T=0.0s** — Prompt back to `❯`.
- **T=0.5s** — Type `au oops --session last` (0.8s).
- **T=1.3s** — Enter. Prompt appears listing every file touched — expect 30–80 files. Hold 1.2s.
- **T=2.5s** — Press `Y`.
- **T=3.0–4.2s** — Output stream: `restored 67 file(s) in 312ms`.
- **T=4.2–5.0s** — Type `git status` and Enter. Output: `nothing to commit, working tree clean`. END.

### Splice

```sh
# 2.5x speed the destructive phase (60s -> 24s), keep restore phase real time
ffmpeg -i phase_a.mp4 -filter:v "setpts=0.4*PTS" phase_a_fast.mp4
ffmpeg -i phase_a_fast.mp4 -i phase_b.mp4 -filter_complex "[0:v][1:v]concat=n=2:v=1[v]" -map "[v]" apocalypse.mp4
```

- Export MP4 as the primary (landing page video, 30s). Generate GIF only if needed for an inline tweet preview, cap at 15 MB:
  ```sh
  gifski -W 900 --fps 20 --quality 85 -o apocalypse.gif apocalypse.mp4
  ```
- Save to `www/public/demos/apocalypse.mp4` and `.gif`.

**Fake-vs-real decision:** Phase A **must be real Claude Code**. The tweet-reply "is this faked?" is the attack vector and the only defense is that it isn't. The only concession to post-production is the 2.5x time-lapse, which we'll disclose in the tweet copy ("2.5x speed").

---

## Demo 3 — `au blame` static screenshot (PNG)

Not a GIF. One perfect frame.

### Setup

- Use `/tmp/au-demo` with an extra file `src/config.rs` seeded via three separate hook sessions so attribution is real:
  1. `AU_AGENT=initial-scan au hook write src/config.rs < initial_config.txt`
  2. `AU_AGENT=claude-code AU_SESSION=claude-a au hook write src/config.rs < claude_edit.txt`
  3. `AU_AGENT=cursor AU_SESSION=cursor-b au hook write src/config.rs < cursor_edit.txt`
  4. One manual edit with `$EDITOR` to get an `unknown` line.
- The committed test fixture in the binary already produces this exact mixed-attribution output — reuse it if faster.

### The frame

Terminal at 1280x720, dark true-black, prompt at top. Command typed:
```
❯ au blame src/config.rs
```
Output (the money shot — match tweet #4 exactly):
```
cursor        cursor-b   2026-04-08 08:33   1: pub const PORT: u16 = 9090;
initial-scan  -          2026-04-08 08:33   2: pub const HOST: &str = "localhost";
initial-scan  -          2026-04-08 08:33   3: pub const DEBUG: bool = false;
claude-code   claude-a   2026-04-08 08:33   4: pub const TIMEOUT: u32 = 30;
unknown       -          2026-04-08 08:33   5: pub const VERSION: &str = "1.0";
```
Agent column colored: cursor = magenta, claude-code = orange, initial-scan = dim, unknown = yellow.

### Capture

- macOS `Cmd+Shift+4` → Space → click terminal window. Then crop to **1200x800** in Preview, **no window chrome**, no shadow (`Cmd+Shift+4` with Option held suppresses shadow).
- Export PNG, run through ImageOptim.
- Save as `www/public/demos/blame.png` and `assets/readme/blame.png`.

---

## Demo 4 — `au tui` timeline scrubber (15s)

The "wait you can do that" reply-bait.

### Setup

- Use `/tmp/au-demo` after running Demos 1-3 so there are ≥20 timeline events across agents.
- Optional: pre-populate more history with a loop writing to `src/auth.rs` under different `AU_AGENT=` values to give visibly varied rows.

### Frame-by-frame (24fps, 15.0s)

- **T=0.00–1.00s** — Prompt at `❯`. Type `au tui` (0.5s). Enter.
- **T=1.00–2.20s** — TUI paints. Left pane: event list, newest at top, 15 rows visible. Right pane: diff of selected event. Bottom status bar: `↑↓ scrub  ⏎ restore  q quit`.
- **T=2.20–4.00s** — Press `↓` five times, one press every 0.35s. Each press: selection highlight moves, right pane redraws the diff for that event. Agent badges flicker (claude-code → cursor → initial-scan).
- **T=4.00–5.00s** — Hold on a `claude-code` event. Right pane shows a red `-` / green `+` diff of `src/auth.rs`.
- **T=5.00–7.00s** — Press `↑` three times slowly. Diff redraws each time.
- **T=7.00–8.50s** — Press `→` to expand the event — side panel shows session metadata: pid, tool, hash, bytes.
- **T=8.50–10.50s** — Press `←` then page-down (`PgDn`). Jump 10 events forward, new diff renders — a multi-file batch (`tests/auth.rs` + `src/lib.rs`).
- **T=10.50–12.50s** — Press `/` to filter. Type `claude` (0.6s). List collapses to claude-code events only. Hold.
- **T=12.50–14.00s** — Press Esc to clear filter. Full list returns.
- **T=14.00–15.00s** — Press `q`. TUI exits cleanly to `❯`. END.

### Post

- Export MP4, convert:
  ```sh
  ffmpeg -i tui.mp4 -vf "fps=24,scale=1000:-1:flags=lanczos" tui.mp4.tmp && mv tui.mp4.tmp tui.mp4
  gifski -W 1000 --fps 24 --quality 90 -o tui.gif tui.mp4
  ```
- Target under 4 MB.
- Save as `www/public/demos/tui.gif` and `.mp4`.

---

## Tools you'll need

| Tool | Purpose | Link |
|---|---|---|
| **Kap** | Free screen recorder, good for GIFs ≤ 30s | https://getkap.co |
| **Screen Studio** | Higher-quality MP4s, smooth cursor, auto-zoom | https://screen.studio |
| **ffmpeg** | MP4 ↔ GIF conversion, palette optimization, concat, speed change | `brew install ffmpeg` |
| **gifski** | Best-in-class GIF palette generation (better than ffmpeg alone) | `brew install gifski` |
| **ImageOptim** | Lossless GIF/PNG compression, drag-and-drop | https://imageoptim.com |
| **Rectangle** | Pin terminal to exact 1280x720 | `brew install --cask rectangle` |

### Canonical ffmpeg incantations

High-quality palette-optimized GIF (use when gifski unavailable):
```sh
ffmpeg -i in.mp4 -vf "fps=24,scale=960:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer:bayer_scale=5" -loop 0 out.gif
```

Time-lapse 2.5x for the apocalypse phase A:
```sh
ffmpeg -i phase_a.mp4 -filter:v "setpts=0.4*PTS" -an phase_a_fast.mp4
```

Concat without re-encoding:
```sh
ffmpeg -f concat -safe 0 -i list.txt -c copy out.mp4
```

Hard file-size cap via target bitrate (for tweet inline, 15 MB cap):
```sh
ffmpeg -i in.mp4 -b:v 2M -maxrate 2M -bufsize 4M out.mp4
```

gifski (preferred for final GIFs):
```sh
gifski -W 960 --fps 24 --quality 90 -o out.gif frame_%04d.png
# or from mp4:
gifski -W 960 --fps 24 --quality 90 -o out.gif in.mp4
```

---

## Final output map

```
www/public/demos/oops.gif         (Demo 1, ≤2 MB, README + tweet #2)
www/public/demos/oops.mp4         (Demo 1, landing page hero)
www/public/demos/apocalypse.mp4   (Demo 2, landing + tweet #9)
www/public/demos/apocalypse.gif   (Demo 2, only if needed inline)
www/public/demos/blame.png        (Demo 3, 1200x800, tweet #4 + README)
www/public/demos/tui.gif          (Demo 4, ≤4 MB, reply-bait)
www/public/demos/tui.mp4          (Demo 4, landing page)
assets/readme/oops.mp4            (GitHub README video embed)
assets/readme/blame.png           (GitHub README static)
```
