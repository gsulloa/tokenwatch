## Context

Cost is computed once, at ingest time, in `ingest_file` (`src/ingest/mod.rs`):
`let cost = pricing::cost(&model, &usage);` and written to `usage_events.cost`.
Ingestion is incremental — `ingest_files` tracks `(size, mtime, lines_ingested)`
per JSONL file and skips files that haven't grown — so once a Sonnet 5 line is
ingested it is never re-parsed. PR #34's fix to `price_row` therefore only
affects rows ingested *after* the fix; the ~1 week of Sonnet 5 events already in
SQLite keep `cost = 0`.

The DB already has an established pattern for exactly this kind of repair:
schema-version migrations in `src/db/mod.rs` that run an **idempotent backfill**
over stored rows. `apply_v2/v3/v4` all call `backfill_project_names`, which reads
each row's stored `project_path` and rewrites the derived `project_name` in a
single transaction. Cost is analogous: it is a pure function of the stored
`model` + four token columns, all of which are already persisted, so it can be
recomputed without touching the JSONL source or ingest offsets.

Current `schema_version` is `5`. Migrations run automatically on every
`open_at` → `migrate`.

## Goals / Non-Goals

**Goals:**
- Repair the persisted `cost` of all pre-fix events (Sonnet 5 → correct, non-zero
  cost), so weekly/historical totals are accurate after the app updates.
- Do it automatically on next launch, with no user action and no re-ingestion.
- Be idempotent and deterministic — safe to run on already-correct data.
- Establish the general contract: a price-table change reconciles stored cost,
  not just new ingests.

**Non-Goals:**
- No re-parsing of JSONL, no change to `ingest_files` offsets or dedup keys.
- No change to token counts, `project_name`, or any column other than `cost`.
- No change to the price table values themselves (PR #34 already fixed matching).
- No UI change — the charts/aggregations already read `cost` from the DB.

## Decisions

**D1 — Backfill via a new schema migration (`v6`) rather than a one-off command
or re-ingest.** Mirrors the existing `backfill_project_names` precedent: runs
once, tracked by `schema_version`, automatic on launch, no user-facing surface.
- *Alternative rejected — force re-ingest by clearing `ingest_files`:* would
  re-parse every JSONL, is slow, risks the source files having been rotated/
  deleted (SQLite is the source of truth by design), and wouldn't repair events
  whose JSONL no longer exists. The whole point of persisting cost is to survive
  log deletion, so recompute must work from the DB alone.
- *Alternative rejected — recompute on read (in the aggregation queries):* would
  move a pure-function cost into every query, diverge from the "cost persisted at
  ingest" model, and require passing the price table into the query layer.

**D2 — Recompute `cost` for every row unconditionally, not only rows where
`cost = 0`.** `pricing::cost(model, usage)` is the single source of truth; setting
every row to it repairs zero-cost Sonnet 5 events *and* any other row whose price
row changed, and is trivially idempotent. Filtering on `cost = 0` would miss rows
that were mispriced to a non-zero-but-wrong value and adds no real speed benefit
at this data scale (per-user local DB).

**D3 — Reuse `pricing::cost` and `pricing::Usage` directly.** They are already
`pub(crate)`-visible from `db` (same crate). The migration reads `model`,
`input_tokens`, `output_tokens`, `cache_creation_tokens`, `cache_read_tokens`,
builds a `pricing::Usage`, and calls `pricing::cost`. No duplication of pricing
logic in the migration.

**D4 — Collect-then-update in a single transaction**, following
`backfill_project_names` exactly: `SELECT` all `(dedup_key, model, tokens…)` into
a Vec, open one `unchecked_transaction`, `UPDATE ... SET cost = ?1 WHERE
dedup_key = ?2` per row, commit. Atomic; a crash mid-migration leaves
`schema_version` at 5 so it re-runs cleanly next launch.

**D5 — Version bump written as the literal `"6"`** (consistent with the existing
`set_meta(conn, "schema_version", "N")` calls). `apply_v6` is pure DML and, like
`apply_v5`, can be wrapped by the caller in `unchecked_transaction`; but since
`recompute_costs` opens its own transaction (like `backfill_project_names`), it
is called directly and the version set immediately after — matching the `v4`
handling note in `migrate`.

## Risks / Trade-offs

- **[A future price-table value change would silently re-baseline all historical
  cost on next launch]** → Acceptable and arguably desired: cost becomes a value
  reconciled against the current table. Documented in the modified
  `usage-ingestion` requirement so it is an intentional contract, not a surprise.
- **[Recompute runs over the full table on the launch that crosses v5→v6]** →
  One-time, single transaction, local per-user DB (thousands of rows, not
  millions). Same cost profile as the existing `backfill_project_names` that
  already runs over every row. Negligible.
- **[Unknown-family models still yield cost 0]** → Correct by design;
  `pricing::cost` returns 0 for unknown families and logs. The backfill doesn't
  invent prices, it only applies the current table.
- **[Idempotency depends on `pricing::cost` being pure]** → It is (no I/O, no
  state); a regression test asserts a second run of `apply_v6` leaves values
  unchanged, mirroring the existing `apply_v2` idempotency test.

## Migration Plan

1. Ship `apply_v6` + `recompute_costs` and bump `CURRENT_SCHEMA_VERSION`/the
   `version < 6` block in `migrate`.
2. On the user's next app launch, `open_default` → `migrate` detects
   `schema_version = 5 < 6`, runs `recompute_costs`, sets version to `6`.
3. The next background refresh emits `usage-updated`; charts re-read corrected
   `cost`. (A refresh happens on launch anyway.)
4. **Rollback**: none needed for data (only `cost` recomputed from stored inputs;
   values are re-derivable). Reverting the binary leaves the DB at v6 with correct
   costs; an older binary ignores unknown-but-higher versions safely per the
   existing `migrate` guard (it only *applies* missing lower versions).

## Open Questions

- None. Scope, precedent, and data model are all established.
