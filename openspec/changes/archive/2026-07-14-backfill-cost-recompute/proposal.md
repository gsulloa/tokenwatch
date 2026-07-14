## Why

PR #34 fixed the price matcher so `claude-sonnet-5` (and future major versions)
resolve to their family's price row, but cost is persisted per-event at ingest
time. Because JSONL files are ingested incrementally and never re-read, every
Sonnet 5 event stored *before* the fix — roughly the last week of usage — remains
in SQLite with `cost = 0` and keeps under-reporting the weekly/historical cost.
New ingests are correct; the already-stored history is not.

## What Changes

- Add a schema migration (`v6`) that recomputes and rewrites the `cost` column
  for **all** existing `usage_events` rows from their already-stored `model` and
  token counts, using the current family-based `pricing::cost`. This backfills
  the zero-cost Sonnet 5 events (and repairs any other row whose price table
  entry has since changed).
- The recompute is deterministic and idempotent: every row's cost becomes
  `pricing::cost(model, usage)`, so re-running the migration is a no-op and rows
  already correct are left effectively unchanged.
- No re-ingestion of JSONL is required and no `ingest_files` offsets are touched;
  the fix operates purely on stored token data.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `usage-ingestion`: the "Cálculo de costo estimado" behavior gains a
  requirement that a change to the embedded price table (or its matching rules)
  MUST repair the persisted cost of already-stored events, not only affect new
  ingests. This makes cost a value that is reconciled against the current price
  table rather than frozen at first ingest.

## Impact

- **Code**: `packages/app/src-tauri/src/db/mod.rs` — new `apply_v6` migration +
  `recompute_costs` helper and version bump to 6; `packages/app/src-tauri/src/pricing.rs`
  (reused, no change expected — `cost`/`Usage` already public to the crate).
- **Data**: in-place `UPDATE` of the `cost` column on the local `tokenwatch.db`;
  runs once automatically on next app launch. No schema shape change, no data loss.
- **User-visible**: weekly / historical cost for Sonnet 5 (and any previously
  mispriced model) becomes correct after the app updates and reopens the DB.
- **Out of scope**: token counts, dedup keys, `project_name`, and ingest offsets
  are untouched.
