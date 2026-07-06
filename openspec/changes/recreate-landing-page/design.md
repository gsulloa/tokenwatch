## Context

The landing SPA lives at `packages/infra/lib/LandingStack/app/` (Vite + React
18 + TypeScript, custom CSS, Geist fonts). It is built at CDK synth time via
`pnpm run landing:build` and served from S3 + CloudFront at
`tokenwatch.gulloa.click`. The marketing content in `src/App.tsx` (~830 lines)
and the meta tags in `index.html` currently describe a database inspection tool
("A hundred eyes on every database") — an unrelated template that has nothing to
do with the actual product. Legal content lives separately in `src/legal.tsx`
(`PrivacyPolicy`, `TermsOfService`) with `/privacy` and `/terms` routes.

The real app (verified in `packages/app`) is a macOS menu-bar client that
ingests `~/.claude/projects/**/*.jsonl`, computes per-project/per-model token
and USD cost, renders an interactive dashboard (stacked-area charts, KPI cards,
date-range filters, detail table), and a menu-bar popover showing live 5h
session + 7-day weekly usage limits (from Anthropic's OAuth usage API via
Keychain), per-model weekly breakdowns, today-by-project usage, project-group
budgets with share/USD caps, threshold notifications, and auto-updates.

## Goals / Non-Goals

**Goals:**
- Replace 100% of the marketing copy and product mockup in `App.tsx` and the
  meta/title/description in `index.html` with accurate TokenWatch content.
- Keep the existing design language (dark theme, violet accent `#a855f7`,
  Geist/Geist Mono, animations, responsive breakpoints) so the rewrite feels
  like a polished evolution, not a regression.
- Preserve the runtime release-manifest fetch (`/releases/download.json`) that
  populates version + build date.
- Rewrite the root `README.md` to reflect the shipped app.

**Non-Goals:**
- No changes to `src/legal.tsx`, the `/privacy` or `/terms` routes, or any
  legal text.
- No changes to CDK infrastructure (`LandingStack/index.ts`), build scripts, or
  deployment.
- No new dependencies, no CSS framework migration, no new fonts.
- Not shipping platform builds the app does not distribute (macOS-first).

## Decisions

### Rewrite `App.tsx` in place, reuse the section scaffold
Keep the existing component architecture (sticky nav, hero, sections, download
grid, footer, scroll-reveal + OS-detection hooks) and swap the *content*. This
minimizes CSS churn and preserves working behaviors (manifest fetch, reveal
animations, routing). Alternative — greenfield rewrite of the whole app dir —
rejected: higher risk, throws away working nav/routing/animation plumbing for
no benefit.

### New content architecture (sections)
- **Hero**: headline on watching Claude token spend from the menu bar; lede
  covering automatic ingestion + cost/limits; primary "Download for macOS" CTA
  + version/date note; mockup showing the popover (usage-limit gauges) and/or
  dashboard chart.
- **"How it works" / Sources**: reframe the old 6-source grid into TokenWatch's
  real data flow — reads `~/.claude/projects/**/*.jsonl`, dedups, ingests every
  30s, aggregates by project/model, prices Opus/Sonnet/Haiku. (Card count is
  flexible; content must be accurate.)
- **Console → Dashboard**: wide mockup of the analytics dashboard (stacked-area
  chart, KPI cards, date-range presets, detail table).
- **Features (bento)**: menu-bar popover with live session/weekly limits;
  per-model weekly breakdown; project-group budgets with % / USD caps;
  threshold notifications with mute; interactive charts; auto-updates. Each
  bento cell = one real feature.
- **Download**: macOS-first. Keep the manifest-driven version/date. Present the
  macOS build(s) as primary; do not present unsupported platforms as available.
- **Footer**: keep Privacy/Terms links; update section anchors to the new IDs.

### Mockup approach
Rebuild the in-page mockup with the same CSS-driven, no-image technique already
used (styled divs / SVG), depicting real UI: colored usage-limit gauge bars
(blue/yellow/red thresholds matching the app), a stacked-area-style chart block,
and KPI/legend chrome. Reason: consistent with current build (no asset
pipeline), keeps bundle light, avoids needing real screenshots. Alternative —
embed real PNG screenshots — rejected for now to avoid asset/versioning churn;
can be a follow-up.

### Copy language
Match the app's Spanish-localized UI feel but keep marketing copy consistent
with the current landing's tone. Since the app UI is Spanish and the audience is
the developer using Claude, write the landing primarily in **Spanish** to match
product + user (this also matches the user's request context). Keep terms like
"Opus/Sonnet/Haiku", "tokens", "cache" as-is.

### README rewrite
Change the "Current state" section from "Scaffold only … NOT implemented yet" to
an accurate feature list. Preserve the structure/conventions/build sections
(pnpm workspace, Node 22, Rust stable, the `pnpm typecheck && lint && test:run`
+ cargo gates). Keep it concise.

## Risks / Trade-offs

- **Stale/placeholder facts leaking into copy** → Ground every claim in the
  verified feature inventory; avoid promising unimplemented items (CSV export,
  Slack/email alerts, Windows/Linux, team features).
- **CSS breakage from restructured DOM** → Reuse existing class names/section
  structure where possible; add new classes only for genuinely new mockup
  pieces; verify responsive breakpoints (760/560/880px) still hold.
- **Accidentally touching legal** → `legal.tsx` and its routes are explicitly
  out of scope; only reference them from nav/footer.
- **Download section over-promising platforms** → Present macOS as the shipped
  target; keep OS-detection graceful. Trade-off: less "universal" than the old
  4-tile grid, but honest.
- **Build gate** → After edits, run `pnpm run landing:build` (and repo
  typecheck/lint) to confirm the SPA still compiles before landing.

## Migration Plan

1. Edit `App.tsx`, `index.html`, and `styles.css` (additive) with new content.
2. Rewrite root `README.md`.
3. Run `pnpm run landing:build` + `pnpm typecheck && pnpm lint` to verify.
4. Deploy is unchanged: normal `LandingStack` synth/deploy publishes to S3 +
   CloudFront. Rollback = revert the commit and redeploy (static site, instant).

## Open Questions

- Should the landing be fully Spanish (matches app + user) or bilingual?
  Assumed **Spanish** per above; revisit if the user prefers English/EN-first.
- Include real screenshots now or keep CSS mockups? Assumed **CSS mockups**;
  screenshots can be a follow-up.
