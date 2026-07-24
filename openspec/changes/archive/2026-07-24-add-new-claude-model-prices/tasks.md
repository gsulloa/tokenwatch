## 1. Price table — add the Fable/Mythos tier

- [x] 1.1 In `packages/app/src-tauri/src/pricing.rs`, add `const FABLE: PriceRow = PriceRow::new(10.00, 50.00, 12.50, 1.00);` next to the existing `OPUS`/`SONNET`/`HAIKU` constants, with a comment noting it covers Claude Fable 5 and Claude Mythos 5 (same tier, same prices) and that cache write/read follow the module's 1.25×/0.1× convention.
- [x] 1.2 In `price_row`, add two arms after the `haiku` arm: `else if model.contains("fable") || model.contains("mythos") { Some(FABLE) }`. Verified no shadowing against the existing arms — `fable`/`mythos` share no substring with `opus`/`sonnet`/`haiku`.
- [x] 1.3 Update the module-level doc comment: bumped the "verified" date to 2026-07-24 and added the Fable/Mythos premium tier to the documented price list.

## 2. Migration v7 — reconcile stored Fable costs

- [x] 2.1 In `packages/app/src-tauri/src/db/mod.rs`, added `fn apply_v7(conn: &Connection) -> SqlResult<()>` that calls the existing `recompute_costs(conn)`, with a doc comment explaining it repairs events priced at 0 because their family (Fable/Mythos) was absent from the table.
- [x] 2.2 In `migrate`, added `if version < 7 { apply_v7(conn)?; set_meta(conn, "schema_version", "7")?; }` after the v6 block, **not** wrapped in an outer transaction (`recompute_costs` opens its own) — same note as v4/v6.
- [x] 2.3 Bumped the `schema_version` assertion in `db/mod.rs`'s `test_schema_creation` from `"6"` to `"7"`.
- [x] 2.4 Bumped the `schema_version` assertion in `budgets/mod.rs` (`test_v5_tables_exist_after_migration`, ~line 689) from `"6"` to `"7"`.

## 3. Tests

- [x] 3.1 Extended `test_all_model_variants_resolve` with `claude-fable-5`, `claude-mythos-5`, `claude-mythos-preview`.
- [x] 3.2 Added `test_cost_fable_known_fixture`: `input=1000, output=100, cache_creation=500, cache_read=2000` → `0.01 + 0.005 + 0.00625 + 0.002 = 0.02325`, asserted within `1e-9`.
- [x] 3.3 Extended `test_pricing_source_documented` with the Fable row's four values (10.00 / 50.00 / 12.50 / 1.00) and a check that `price_row("claude-mythos-5")` shares the row.
- [x] 3.4 Added `test_v7_recompute_fixes_zero_cost_fable`: inserts a `claude-fable-5` row with `cost = 0`, runs `apply_v7`, asserts `cost == pricing::cost(...)` and `> 0`.
- [x] 3.5 Added `test_v7_recompute_idempotent`: runs `apply_v7` twice, asserts cost unchanged on the second run.
- [x] 3.6 Confirmed `test_cost_unknown_model_returns_zero` still passes — `gpt-4o` stays at 0.

## 4. Validation

- [x] 4.1 `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` zero warnings; `cargo test` **105 passed / 0 failed**.
- [x] 4.2 `pnpm typecheck` clean; `pnpm lint` clean; `pnpm test:run` **19 files / 200 tests passed**. No frontend change was needed.
- [ ] 4.3 Run `openspec validate add-new-claude-model-prices --strict`. **BLOCKED**: the `openspec` CLI is not installed in this environment (`npx openspec` also fails to resolve an executable). Change artifacts were authored to match the conventions of the archived changes. Re-run once the CLI is available.
- [x] 4.4 Applied the delta into `openspec/specs/usage-ingestion/spec.md` on archive — the "Cálculo de costo estimado" requirement now covers the `fable`/`mythos` premium tier, alias/provider-prefix tolerance, and the CI guard, with four new scenarios. Matched the surrounding no-blank-line-after-heading style of that section to keep the diff minimal.

## 5. Manual verification

- [x] 5.1 Verified end-to-end against a **copy** of the real `tokenwatch.db` (live DB never mutated; source read read-only). Confirmed via a temporary integration harness calling the real `db::open_at`: `schema_version` went **6 → 7**, and the 297 stored `claude-fable-5` events went from **$0.0000 → $75.6626** — matching an independent SQL projection of the same formula to the cent. Row-level join against a pre-migration snapshot confirmed the spec's "only the `cost` column changes" invariant on all shared rows: **0** rows with any change to the four token counts, `total_tokens`, `project_name`, or `timestamp`, and **0** non-Fable rows with a cost change. Harness deleted afterwards; `git status` shows only the 3 intended source files. *Not covered:* the GUI was not launched, so the chart/table render of the corrected value is unconfirmed visually — the data layer is.
- [x] 5.2 Confirmed `claude-fable-5` no longer hits the unknown-model path (`price_row` returns `Some(FABLE)`, asserted by `test_all_model_variants_resolve`), so it emits no `unknown model — cost set to 0` warning. `effort level` (2 events) still warns, which is correct — it is not a model.

## 6. Findings that changed the plan (no code required)

- [x] 6.1 **Claude Opus 5 needs no price row.** Verified against platform.claude.com on 2026-07-24: Opus 5 is **$5 / $25 per MTok, $6.25 cache write, $0.50 cache read — identical to Opus 4.5–4.8**. The real DB confirms it already prices correctly (42–48 `claude-opus-5` events at ~$10.20 total, never 0). Adding a dedicated `OPUS_5` row would duplicate four identical numbers. Recorded as D6 in `design.md`.
- [x] 6.2 **Sonnet stays at list price** ($3/$15) per the user's decision; no date-windowed pricing added. Recorded in D3 and closed as an open question.
