# agent-undo www

The launch landing site for [agent-undo](https://github.com/peaktwilight/agent-undo) — Ctrl-Z for AI coding agents.

## Stack

- **Astro 6** (static output, single `/` route)
- **React 19** islands (only `DockNav`, hydrated `client:load`)
- **Tailwind CSS v4** via `@tailwindcss/vite` (no config file — `@theme` block lives in `src/styles/global.css`)
- **@fontsource-variable/inter** for body, **Outfit** + **JetBrains Mono** from Google Fonts for display & code
- **tw-animate-css** for fade-up keyframes
- No analytics. No trackers. No third-party JS beyond Google Fonts.

## Develop

```bash
nvm use          # uses .nvmrc if you have nvm
pnpm install
pnpm dev          # http://localhost:4321
```

Astro 6 in this repo requires Node `>=22.12.0`. CI uses Node 22, and local
builds on Node 18 will fail before Astro starts.

## Build

```bash
pnpm build        # → ./dist
pnpm preview      # serve the built site locally
```

The build is fully static — `dist/` is a flat HTML/CSS/JS tree you can drop on any static host.

## Deploy

The site is designed to drop straight onto:

- **Netlify** — point at this repo, build command `pnpm build`, publish dir `dist`. The early-access form uses `data-netlify="true"`, so it works out of the box on Netlify.
- **Cloudflare Pages** — same settings.
- **Vercel** — same settings; for the email form, swap to a Vercel-friendly endpoint or wire up Formspree.
- **Anything that serves static files** — S3, GitHub Pages, your own nginx.

## Project layout

```
www/
├── astro.config.mjs        # Astro + @tailwindcss/vite + react integrations
├── package.json
├── tsconfig.json
├── public/
│   ├── favicon.svg
│   └── robots.txt
└── src/
    ├── components/
    │   └── DockNav.tsx     # the floating top pill (only React island)
    ├── layouts/
    │   └── Layout.astro    # <head>, fonts, OG/Twitter meta
    ├── lib/
    │   └── cn.ts           # tiny clsx + tailwind-merge helper
    ├── pages/
    │   └── index.astro     # the entire landing page
    └── styles/
        └── global.css      # @theme tokens + cyan grid + fade-up keyframes
```

## Theme

All colors live as CSS variables in `src/styles/global.css` under `@theme` (Tailwind v4). The accent is **cyan-500 `#06b6d4`** — the "safety / rewind" color. Replace any reference to it there to re-skin the site.

## Notes

- Only the `DockNav` component is hydrated. The rest of the page is static HTML — including the copy buttons, which use a single inline `<script>` event delegate.
- If you add new pages, also add a sitemap (the `@astrojs/sitemap` integration is not yet wired up).
