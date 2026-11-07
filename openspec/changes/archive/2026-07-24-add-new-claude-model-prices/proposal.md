## Why

The embedded price table only knows three model families — `opus`, `sonnet`,
`haiku`. Matching is version-agnostic (PR #34), so new *versions* of a known
family price correctly, but a genuinely **new family name** falls through to
`None` → `cost = 0` + a `tracing::warn!`, and the usage is silently free.

That has already happened. A scan of the local Claude logs
(`~/.claude/projects/**/*.jsonl`) shows **244 assistant events with
`model = "claude-fable-5"`**, all currently persisted at `cost = 0`. Claude
Fable 5 is Anthropic's most capable widely released model and is priced *above*
the Opus tier ($10/$50 per MTok), so these are the most expensive events in the
database and they are being counted as free. `claude-mythos-5` is the same tier
at identical pricing (Project Glasswing) and would fail the same way.

The failure mode is silent by design — cost 0 plus a log line the user never
reads — so the table drifts behind Anthropic's lineup without any visible signal.

## What Changes

- Add a `FABLE` price row ($10.00 input / $50.00 output / $12.50 cache write /
  $1.00 cache read per MTok) to the embedded table in `pricing.rs`, and match it
  from the family names `fable` and `mythos` (same tier, same prices, different
  distribution channel).
- Add a schema migration (`v7`) that reruns the existing `recompute_costs`
  reconciliation, so the 244 already-stored Fable events get their real cost
  instead of 0 — the contract established by the `v6` migration.
- Extend `test_all_model_variants_resolve` to cover `claude-fable-5` and
  `claude-mythos-5`, so the guard test fails if a shipped family is missing.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `usage-ingestion`: the "Cálculo de costo estimado" requirement enumerates the
  families the embedded table MUST cover, and gains the premium tier (`fable` /
  `mythos`) alongside `opus` / `sonnet` / `haiku`. The existing reconciliation
  requirement already obliges a table change to repair stored history; this
  change exercises it.

## Impact

- **Code**: `packages/app/src-tauri/src/pricing.rs` — new `FABLE` const, two new
  arms in `price_row`, updated module doc + pricing-source test;
  `packages/app/src-tauri/src/db/mod.rs` — `apply_v7` (calls the existing
  `recompute_costs`) and the `version < 7` block; `schema_version` assertions in
  `db/mod.rs` and `budgets/mod.rs` bump from `"6"` to `"7"`.
- **Data**: in-place `UPDATE` of `cost` on the local `tokenwatch.db`, once, on
  next launch. No schema shape change; token counts and every other column
  untouched.
- **User-visible**: Fable 5 usage stops reading as $0 — historical and weekly
  cost totals go up by the real Fable spend. Charts and the model table need no
  change (they render whatever model names the DB returns).
- **Out of scope**: no per-token-price change to `opus` / `sonnet` / `haiku`; no
  date-windowed pricing (see D3 — Sonnet 5 introductory rates are deliberately
  not modelled, per the user's decision to keep Sonnet at list price); no UI work;
  `<synthetic>` records are already skipped at ingest.
- **Verified needing no change (D6)**: Claude Opus 5 is $5/$25 per MTok —
  identical to Opus 4.5–4.8 — so the 35 stored `claude-opus-5` events already
  price correctly through the existing `OPUS` row. Confirmed against the live
  pricing page on 2026-07-24.
