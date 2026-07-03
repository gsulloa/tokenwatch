# TokenWatch

A macOS menu-bar app for monitoring Claude / Codex token usage per project and workspace.

> **Status:** scaffold. The Tauri + React shell and AWS infrastructure are in
> place; token-monitoring features are not implemented yet.

## What it does (planned)

TokenWatch sits in the macOS menu bar and gives you an at-a-glance view of how
many tokens your AI coding sessions are burning:

- **Per-project / per-workspace usage** — aggregates token counts by project.
- **Data sources** — reads from [`ccusage`](https://github.com/ryoppippi/ccusage)
  and from Claude Code's session logs at `~/.claude/projects/**/*.jsonl`.
- **Limit alerts** — warns when a project or a global budget approaches or
  exceeds a configured token limit.
- **Menu-bar native** — a lightweight tray popover, no dock icon.

## Repository layout

- `packages/app`   — Tauri 2 + React desktop app (menu-bar shell).
- `packages/infra` — AWS CDK: release hosting, landing page, analytics, feedback.
- `tokenwatch/`     — context folder placeholder.

## Prerequisites

- Node 22, pnpm 10, Rust stable (with `cargo fmt`, `clippy`).

## Run

    pnpm install
    pnpm tauri:dev

## Build

    pnpm tauri:build

## Release pipeline

Tag pushes (`v*`) trigger `.github/workflows/release.yml`: build + sign +
notarize on macOS/Windows/Linux, then publish artifacts and updater manifests
to S3 + CloudFront (`releases.tokenwatch.app`). Release notes come from
`CHANGELOG.md`.

## Infrastructure

See `packages/infra/README.md`. Stacks: Dns, Analytics, Releases, Landing,
Feedback. `DnsStack` imports an existing Route53 hosted zone — set the real
`HOSTED_ZONE_ID` in `packages/infra/constants.ts` before deploying.
