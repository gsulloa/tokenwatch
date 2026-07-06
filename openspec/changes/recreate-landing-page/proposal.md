## Why

The public landing page at `tokenwatch.gulloa.click` describes a completely
different product than what TokenWatch actually is. The current copy sells a
"precision desktop client for inspecting and editing data across Postgres,
MySQL, SQL Server, DynamoDB, CloudWatch and Athena" ("A hundred eyes on every
database") — a leftover from a template. The real app is a macOS menu-bar
monitor for Claude token usage, costs, and limits. Anyone who lands on the site
and downloads the app gets something unrelated to the marketing, which is
misleading and damages trust. The README has the opposite problem: it still
says the token-monitoring features are "NOT implemented yet" when in fact the
app ships a full dashboard, menu-bar popover, usage limits, and project budgets.

## What Changes

- Rewrite the marketing landing page (`App.tsx`) from scratch so every section
  accurately reflects TokenWatch's real, implemented capabilities: automatic
  ingestion of `~/.claude/projects/**/*.jsonl`, per-project/per-model cost and
  token analytics, interactive time-series charts, the menu-bar popover with
  live session/weekly usage limits, per-model weekly breakdowns, project-group
  budgets with caps, threshold notifications, and seamless auto-updates.
- Replace all hero, sources/features, "console", and download-section copy and
  the in-page product mockup so they depict the actual TokenWatch UI (dashboard
  charts + menu-bar popover) instead of a database grid/SQL editor.
- Update navigation labels, footer links, meta/`<title>`/description, and any
  OS-detection download messaging to match the macOS-only, Claude-focused
  product.
- **KEEP** the legal content unchanged: `PrivacyPolicy` and `TermsOfService`
  in `src/legal.tsx`, their `/privacy` and `/terms` routes, and the footer
  links to them. Legal copy is explicitly out of scope for rewriting.
- Rewrite the root `README.md` so it reflects the current, shipped state of the
  app (features implemented, not "scaffold only") while preserving accurate
  structure, conventions, and build/release instructions.

## Capabilities

### New Capabilities
- `landing-page`: The public marketing website — its content accuracy
  requirements (sections must reflect real app features), the mandatory
  preservation of legal pages/routes, styling/branding continuity, and the
  build/deploy contract via the existing CDK `LandingStack`.

### Modified Capabilities
<!-- No existing spec covers requirement-level behavior of the landing page or README; none change. -->

## Impact

- **Code**: `packages/infra/lib/LandingStack/app/src/App.tsx` (full rewrite of
  marketing content/mockup), `index.html` (meta/title), and
  `packages/infra/lib/LandingStack/app/src/styles.css` (adjust/extend styles as
  needed for the new mockup and sections). `src/legal.tsx` is untouched.
- **Docs**: `README.md` (root) rewritten to current state.
- **Build/Deploy**: No infrastructure change. Landing still builds via
  `pnpm run landing:build` and deploys through the existing `LandingStack`
  (S3 + CloudFront, `tokenwatch.gulloa.click`). The runtime fetch of
  `/releases/download.json` for version/build date is preserved.
- **Dependencies**: None added; keep React + Vite + Geist fonts + custom CSS.
