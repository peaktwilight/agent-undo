# OG image design spec

For `agent-undo.com/og-image.png` and `agent-undo.com/twitter-card.png`. Hand this file to a designer or paste it into Figma AI / Stitch / Recraft / Midjourney for generation.

## Format

- **Dimensions**: 1200 × 630 px (Open Graph + Twitter standard)
- **Format**: PNG, lossless, <500 KB
- **DPI**: 72 (web standard)
- **Safe zone**: 100px margin on all sides — text inside the safe zone

## Brand tokens (paste into Figma color styles or Tailwind config)

```
background:        #0a0a0a
card:              #171717
foreground:        #ffffff
muted:             #a3a3a3
ash:               #737373
border:            #404040

primary:           #06b6d4   (cyan-500)
primary-light:     #22d3ee   (cyan-400)
primary-dark:      #0e7490   (cyan-700)
glow alpha 25%:    #06b6d440
```

## Fonts

- **Headline**: Outfit Bold (Google Fonts) — fallback Inter Bold
- **Mono**: JetBrains Mono Bold — fallback any monospace
- **Body**: Inter Regular

If you can't get Outfit, use Inter Bold for everything.

## Layout — option A (recommended, headline-first)

```
┌──────────────────────────────────────────────┐
│                                              │
│  source control for the code your agent     │  ← eyebrow, 18px, cyan-300, uppercase, 0.2em letter-spacing
│  wrote                                       │
│                                              │
│  git for humans.                             │  ← headline line 1, 88px, white, Outfit Bold
│  au for agents.                              │  ← headline line 2, 88px, gradient cyan-300 → cyan-500, Outfit Bold
│                                              │
│  ┌─────────────────────────────────────┐     │  ← terminal mockup card
│  │ ● ● ●        ~/my-project — au oops │     │     #171717 bg, #404040 border, rounded-xl
│  │                                      │     │     cyan glow shadow
│  │ $ au oops                            │     │
│  │ ⚠ Last agent: claude-code            │     │
│  │ Roll back 5 files? [Y/n] _          │     │
│  └─────────────────────────────────────┘     │
│                                              │
│  agent-undo.com                              │  ← bottom right, 14px, ash, mono
│                                              │
└──────────────────────────────────────────────┘
```

**Background:** #0a0a0a base + cyan dot grid at 4% alpha (matches the live site):
```css
background-image:
  linear-gradient(rgba(6,182,212,0.06) 1px, transparent 1px),
  linear-gradient(90deg, rgba(6,182,212,0.06) 1px, transparent 1px);
background-size: 40px 40px;
```

**Headline glow:** behind the second line ("au for agents."), add a soft cyan radial gradient at 15% alpha, 200px blur radius, to give it "lit from within" feel.

## Layout — option B (terminal-first, screenshot-shaped)

If option A feels too text-heavy, lead with a giant terminal mockup:

```
┌──────────────────────────────────────────────┐
│                                              │
│  ┌────────────────────────────────────────┐  │
│  │ ● ● ●           ~/my-project — au oops │  │
│  │                                         │  │
│  │ $ au oops                               │  │
│  │ ⚠  Last agent action: claude-code,      │  │
│  │    session 14:32–14:34, edited 5 files  │  │
│  │                                         │  │
│  │    src/auth.rs        (-87) +12         │  │
│  │    src/middleware.rs  (-23) +5          │  │
│  │    src/lib.rs         (-4) +1           │  │
│  │    tests/auth.rs      (deleted)         │  │
│  │    Cargo.toml         (-2) +0           │  │
│  │                                         │  │
│  │    Roll back this entire session? [Y/n]_│  │
│  └────────────────────────────────────────┘  │
│                                              │
│  git for humans.    au for agents.           │  ← caption underneath, 36px
│  agent-undo.com                              │
│                                              │
└──────────────────────────────────────────────┘
```

This option converts better for technical audiences who want to see "what does the tool actually look like?" before clicking.

## Layout — option C (slogan-only, maximally bold)

The minimalist version. Just the slogan, huge, on the dot grid, with the URL.

```
┌──────────────────────────────────────────────┐
│                                              │
│                                              │
│        git for humans.                       │  ← 120px, white, Outfit Bold
│        au for agents.                        │  ← 120px, gradient cyan, Outfit Bold
│                                              │
│        agent-undo.com                        │  ← 28px, ash, mono
│                                              │
│                                              │
└──────────────────────────────────────────────┘
```

This is the most pretext-shaped option. It bets the slogan alone is the hook. Use this if you trust the slogan to do the work.

## Twitter card variant

Twitter cards render at slightly different sizes (1200×675 for `summary_large_image`). Build option A or C and re-crop to 1200×675 — center vertically, no cropping needed if the safe zone is respected.

## Tools you can use

- **Figma** — copy the brand tokens above into a new file, drop a 1200×630 frame, lay out manually
- **Recraft.ai** — generate the dot-grid background, then layer text in their editor
- **Stitch (stitch.withgoogle.com)** — AI-generated, give it the brand tokens + layout description
- **Midjourney** — generation prompt: *"OG image, dark background, cyan dot grid, large white sans-serif headline 'git for humans. au for agents.', terminal mockup with traffic-light dots, minimalist developer tool branding, 1200x630 aspect ratio"*
- **Manual**: open the live `agent-undo.com` page, screenshot the hero at 1200x630, crop, save as PNG

## Done check

- [ ] PNG is 1200×630 (or 1200×675 for Twitter)
- [ ] File is <500 KB
- [ ] Text is readable when scaled to 600px wide (Twitter feed preview size)
- [ ] No copyrighted imagery
- [ ] Cyan accent matches `#06b6d4`
- [ ] URL `agent-undo.com` appears somewhere
- [ ] Saved as `og-image.png` and `twitter-card.png` in `www/public/`

After dropping the file in `www/public/`, the existing `<meta>` tags in `Layout.astro` will pick it up automatically.

## My pick

**Option C** — the slogan-only, maximally bold version. It's the highest-confidence move. The slogan is the entire pitch. Trust it to carry. The terminal mockup belongs in tweet #2, not the OG card.
