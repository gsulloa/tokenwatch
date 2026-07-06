## 1. Preparation

- [x] 1.1 Re-read the verified feature inventory and confirm which claims are safe to market (exclude stubs: CSV export, Slack/email alerts, Windows/Linux, team features)
- [x] 1.2 Confirm the current `App.tsx` section scaffold, hooks (OS detection, scroll-reveal, manifest fetch), and CSS class names to reuse
- [x] 1.3 Confirm `src/legal.tsx`, `/privacy`, `/terms` routes and footer links — these must remain untouched

## 2. Meta and shell

- [x] 2.1 Update `index.html` `<title>`, meta description, and any OG/social tags to describe TokenWatch (Claude token usage monitor for macOS)
- [x] 2.2 Update the sticky nav labels and section anchors to match the new sections; keep the version pill and download CTA behavior

## 3. Hero section

- [x] 3.1 Rewrite hero tag/headline/lede to communicate monitoring Claude token usage, cost, and limits from the macOS menu bar
- [x] 3.2 Update the primary CTA to "Download for macOS" and keep the manifest-driven version/date note
- [x] 3.3 Rebuild the hero product mockup (CSS/SVG, no images) to depict the real UI: menu-bar popover with usage-limit gauges and/or the dashboard chart

## 4. Feature/content sections

- [x] 4.1 Rewrite the "Sources"/how-it-works section to describe the real data flow: reads `~/.claude/projects/**/*.jsonl`, dedups, ingests every ~30s, aggregates by project/model, prices Opus/Sonnet/Haiku
- [x] 4.2 Rewrite the "Console" section into a Dashboard showcase (stacked-area chart, KPI cards, date-range presets, detail table) with an accurate mockup
- [x] 4.3 Rewrite the bento Features grid so each cell = one real capability: menu-bar popover with live session/weekly limits, per-model weekly breakdown, project-group budgets with %/USD caps, threshold notifications with mute, interactive charts, auto-updates
- [x] 4.4 Remove every reference to databases/SQL/Postgres/MySQL/SQL Server/DynamoDB/CloudWatch/Athena and generic "data inspection"

## 5. Download section and footer

- [x] 5.1 Rewrite the download section to be macOS-first; present the shipped macOS build(s) as primary and avoid promising unsupported platforms as first-class
- [x] 5.2 Keep OS-detection graceful and the release-manifest (`/releases/download.json`) fetch for version + build date
- [x] 5.3 Update the footer: refresh section links to new anchors, keep Privacy/Terms links to `/privacy` and `/terms`, update copyright/version display

## 6. Styling

- [x] 6.1 Add/adjust CSS in `styles.css` for any new mockup/section pieces, reusing the existing dark theme + violet accent + Geist typography
- [x] 6.2 Verify responsive behavior at the existing breakpoints (760px / 560px / 880px) across hero, sections, and download grid

## 7. README

- [x] 7.1 Rewrite root `README.md` "current state" to reflect the shipped app (menu-bar monitoring, dashboard analytics, usage limits, project budgets, auto-updates)
- [x] 7.2 Preserve accurate repository structure, conventions, and build/release instructions; remove "scaffold only / NOT implemented yet" language

## 8. Verification

- [x] 8.1 Run `pnpm run landing:build` and confirm the SPA compiles and produces `dist`
- [x] 8.2 Run `pnpm typecheck && pnpm lint` (and any landing tests) and fix issues
- [x] 8.3 Manually load the built landing and confirm: `/`, `/privacy`, `/terms` render; version/date populate; no database copy remains; responsive layout holds
- [x] 8.4 Confirm `src/legal.tsx` and legal routes are unchanged (git diff shows no legal edits)
