# TokenWatch

A native macOS menu-bar app that monitors Claude token usage, cost, and rate limits per project and workspace — 100% local, no cloud, no telemetry.

## What it does

TokenWatch sits in the macOS menu bar and gives you real-time and historical visibility into how much of your Claude quota is consumed, before you get rate-limited mid-task.

### Menu-bar popover
- **Live session (5h) and weekly limit gauges** — reads from Anthropic's OAuth usage API via macOS Keychain; horizontal rails with threshold ticks at 70/85/100% and a pace marker.
- **Per-model weekly breakdown** — Opus, Sonnet, and Haiku each consume the weekly quota separately; see which model is eating it.
- **Today-by-project list** — ranked projects with share bars, token counts, and percentages.
- **Project-group budgets** — group projects, set caps (% of session or absolute USD), alerts at 50/70/80%.
- **Mute-alerts toggle** and a dashboard shortcut button.

### Dashboard window
- **Stacked-area time-series chart** — by model, project, or combination; 2-3 series with a probe cursor.
- **Date-range presets** — 24h, 3d, 7d, 30d, current month, all-time, plus a custom range picker.
- **Big readouts** — total tokens in range, cost, series count, selected range.
- **Ledger-style detail table** — per-bucket values per series, right-aligned numeric columns, a bold totals row.

### Ingestion engine
- Auto-reads `~/.claude/projects/**/*.jsonl` (Claude Code session logs).
- Deduplicates by message ID, polls every ~30 s with incremental reads (only changed files).
- Aggregates by project + model, normalizes Conductor workspace names.
- Prices Opus / Sonnet / Haiku tokens (including cache hits) to USD using the current Anthropic rate table.

### Other
- **Threshold notifications** — native macOS alerts; globally mutable.
- **Auto-updates** — signed releases via the Tauri updater; always the latest version without friction.

## Repository layout

- `packages/app`   — Tauri 2 + React desktop app (menu-bar).
- `packages/infra` — AWS CDK: release hosting, landing page, analytics, feedback.
- `tokenwatch/`     — context folder.

## Prerequisites

- Node 22, pnpm 10, Rust stable (with `cargo fmt`, `clippy`).

## Run

    pnpm install
    pnpm tauri:dev

## Build

    pnpm tauri:build

## Release pipeline

Releases are tag-driven. `packages/app/scripts/release.sh` is the entrypoint:
it cuts a release branch off `dev`, runs `generate-changelog.mjs` (which fills
the `## [Unreleased]` section of `CHANGELOG.md` from Conventional Commits since
the last tag), bumps the version (`bump-version.mjs`), opens a PR to `master`,
and — after merge — pushes the `vX.Y.Z` tag and back-merges into `dev`.

The tag triggers `.github/workflows/release.yml`: each platform is built,
signed, and (on macOS) notarized. Artifacts, the updater manifest
(`latest.json`), and the download manifest (`download.json`, built by
`build-manifest.mjs`) are published to S3 + CloudFront
(`releases.tokenwatch.gulloa.click`). The GitHub release body is the matching
`## [version]` section sliced from `CHANGELOG.md`. The app surfaces those notes
in a **What's New** dialog the first time it runs a new version, and shows the
current version + an update action in its **About** ("Acerca de") surface.

## Conventions

- pnpm workspace; Node 22; Rust stable.
- `pnpm typecheck && pnpm lint && pnpm test:run` and `cargo fmt && cargo clippy && cargo test` must pass before landing (see PR template and `.github/workflows/release.yml`).

## Infrastructure

See `packages/infra/README.md`. Stacks: Dns, Analytics, Releases, Landing,
Feedback. `DnsStack` imports an existing Route53 hosted zone — set the real
`HOSTED_ZONE_ID` in `packages/infra/constants.ts` before deploying.
