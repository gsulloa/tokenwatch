# TokenWatch

A macOS menu-bar app for monitoring Claude / Codex token usage per project and workspace.

## What it does (target, not yet implemented)

TokenWatch lives in the macOS menu bar (tray) and surfaces real-time and
historical token consumption for AI coding sessions. It reads usage data from
`ccusage` and from `~/.claude/projects/**/*.jsonl`, aggregates by project /
workspace, and raises alerts when configured limits are approached or exceeded.

## Current state

Scaffold only. The Tauri + React shell builds and runs, but the token-monitoring
features are NOT implemented yet. Treat this repo as the base structure.

## Structure

- `packages/app`   — Tauri 2 + React desktop app (menu-bar scaffold).
- `packages/infra` — AWS CDK infrastructure (release hosting, landing, analytics, feedback).
- `tokenwatch/`     — context folder placeholder.

## Conventions

- pnpm workspace; Node 22; Rust stable.
- `pnpm typecheck && pnpm lint && pnpm test:run` and `cargo fmt/clippy/test` must pass before landing (see PR template).

## Design System
Always read DESIGN.md before making any visual or UI decisions.
All font choices, colors, spacing, and aesthetic direction are defined there.
Do not deviate without explicit user approval.
In QA mode, flag any code that doesn't match DESIGN.md.
