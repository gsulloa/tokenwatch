## 1. Migration v6 — recompute persisted cost

- [x] 1.1 In `packages/app/src-tauri/src/db/mod.rs`, add a `recompute_costs(conn)` helper modeled on `backfill_project_names`: `SELECT dedup_key, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens FROM usage_events` into a Vec, then in a single `unchecked_transaction` run `UPDATE usage_events SET cost = ?1 WHERE dedup_key = ?2` where the new cost is `pricing::cost(&model, &pricing::Usage { … })`.
- [x] 1.2 Add `fn apply_v6(conn) -> SqlResult<()>` that calls `recompute_costs(conn)`, with a doc comment explaining it repairs pre-fix zero-cost events (e.g. Sonnet 5) from stored token data.
- [x] 1.3 In `migrate`, add an `if version < 6 { apply_v6(conn)?; set_meta(conn, "schema_version", "6")?; }` block after the v5 block. Do NOT wrap in an outer transaction (recompute_costs opens its own), matching the v4 handling note.
- [x] 1.4 Bump the current-schema-version reference used by the `test_schema_creation` test (expected `"6"`) and any `CURRENT_SCHEMA_VERSION`-style constant if present. (No such constant exists — the literal `"6"` in `migrate` + tests is the source; also bumped a matching assertion in `budgets/mod.rs`.)

## 2. Tests

- [x] 2.1 Add `test_v6_recompute_fixes_zero_cost_sonnet_5`: insert a row with `model = "claude-sonnet-5"`, real token counts, `cost = 0`; run `apply_v6`; assert `cost` equals `pricing::cost("claude-sonnet-5", usage)` and is > 0.
- [x] 2.2 Add `test_v6_recompute_idempotent`: run `apply_v6` twice; assert the cost is unchanged on the second run.
- [x] 2.3 Add `test_v6_recompute_preserves_other_columns`: assert token counts, `total_tokens`, `project_name`, and `dedup_key` are untouched after `apply_v6`.
- [x] 2.4 Add `test_v6_recompute_unknown_model_stays_zero`: insert a row with `model = "gpt-4o"` and `cost = 0`; run `apply_v6`; assert `cost` remains 0.
- [x] 2.5 Update `test_schema_creation` to expect `schema_version = "6"`.

## 3. Spec sync & validation

- [x] 3.1 Run `cargo fmt`, `cargo clippy`, and `cargo test` in `packages/app/src-tauri`; all pass. (fmt --check clean, clippy -D warnings clean, `cargo test` 102 passed / 0 failed.)
- [x] 3.2 Run `openspec validate backfill-cost-recompute --strict` and fix any issues.
- [x] 3.3 On archive, apply the delta into `openspec/specs/usage-ingestion/spec.md` (adds the reconciliation requirement).

## 4. Manual verification

- [ ] 4.1 Against a real `tokenwatch.db` at schema v5 containing Sonnet 5 events with `cost = 0`, launch the app and confirm migration bumps to v6 and the weekly cost for Sonnet 5 becomes non-zero in the chart.
