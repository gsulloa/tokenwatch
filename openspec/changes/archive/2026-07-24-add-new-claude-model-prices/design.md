## Context

`packages/app/src-tauri/src/pricing.rs` holds a three-row embedded table and a
substring matcher:

```rust
const OPUS:   PriceRow = PriceRow::new(5.00, 25.00, 6.25, 0.50);
const SONNET: PriceRow = PriceRow::new(3.00, 15.00, 3.75, 0.30);
const HAIKU:  PriceRow = PriceRow::new(1.00,  5.00, 1.25, 0.10);

fn price_row(model: &str) -> Option<PriceRow> {
    if model.contains("opus") { Some(OPUS) }
    else if model.contains("sonnet") { Some(SONNET) }
    else if model.contains("haiku") { Some(HAIKU) }
    else { None }
}
```

`cost()` returns `0.0` + `tracing::warn!` for `None`. That design choice —
deliberately version-agnostic so `claude-sonnet-5` prices like `claude-sonnet-4-6`
— covers new *versions* but not new *family names*.

Evidence from the local logs (distinct `message.model` values, by volume):

| model | events | resolves today? |
|---|---:|---|
| `claude-opus-4-8` | 57,830 | ✅ opus |
| `claude-sonnet-5` | 33,484 | ✅ sonnet |
| `claude-sonnet-4-6` | 11,208 | ✅ sonnet |
| `claude-haiku-4-5-20251001` | 8,501 | ✅ haiku |
| `sonnet` (bare alias) | 708 | ✅ sonnet |
| `<synthetic>` | 614 | n/a — skipped at ingest (`ingest/mod.rs:285`) |
| **`claude-fable-5`** | **244** | ❌ **`None` → cost 0** |
| `claude-opus-4-7` | 163 | ✅ opus |
| `opus` (bare alias) | 61 | ✅ opus |
| `claude-opus-5` | 35 | ✅ opus |
| `global.anthropic.claude-sonnet-4-6` | 11 | ✅ sonnet (Bedrock prefix, substring still hits) |
| `effort level` | 2 | ❌ malformed — cost 0 (correct outcome) |

Fable is the only real, priced model missing. Its list price is $10/$50 per MTok
— **double Opus** — so those 244 events are the highest-value rows in the DB and
they are recorded as free.

Cost is persisted per event at ingest (`ingest/mod.rs:319`) and never recomputed
on read, so fixing the table alone would only affect future ingests. The `v6`
migration already established the repair pattern: `recompute_costs(conn)` reads
`model` + the four token columns from every row and rewrites `cost` via
`pricing::cost`, in one transaction, idempotently. Current `schema_version` is
`6`.

## Goals / Non-Goals

**Goals:**
- Price Claude Fable 5 (and Claude Mythos 5) correctly at $10/$50 per MTok.
- Repair the 244 already-stored zero-cost Fable events on next launch, with no
  re-ingestion and no user action.
- Keep the version-agnostic family matcher intact — this adds a family, it does
  not change the matching strategy.
- Leave a guard test that fails loudly if a family Claude Code actually emits is
  absent from the table.

**Non-Goals:**
- No change to `opus` / `sonnet` / `haiku` per-token values.
- No date-windowed / introductory pricing (D3).
- No per-model-ID rows — matching stays at family granularity (D2).
- No new UI, no new surface for "unknown model" warnings (D5).
- No handling of the `effort level` garbage string — cost 0 is already correct
  for a non-model.

## Decisions

**D1 — One `FABLE` row, matched from both `fable` and `mythos`.** Claude Fable 5
and Claude Mythos 5 are documented as the same capabilities *and the same
pricing* ($10/$50), differing only in distribution (Mythos 5 is Project
Glasswing-only, succeeding `claude-mythos-preview`). One row keyed by two family
substrings avoids a duplicate constant that could drift apart.

```rust
const FABLE: PriceRow = PriceRow::new(10.00, 50.00, 12.50, 1.00);
```

Cache multipliers follow the module's documented convention: write = 1.25×
input = $12.50, read = 0.1× input = $1.00. Consistent with all three existing
rows, so no new pricing concept is introduced.

- *Alternative rejected — a separate `MYTHOS` const:* identical values, twice the
  maintenance surface, and the naming already tells the reader they're one tier.

**D2 — Keep family-substring matching; do not move to per-model-ID rows.** The
substring approach is why `claude-opus-5` and the bare `sonnet` alias already
price correctly, and why a Bedrock-prefixed `global.anthropic.claude-sonnet-4-6`
works with no extra code. Exact-ID matching would have to enumerate every alias,
snapshot suffix, and provider prefix, and would regress to cost 0 the moment
Anthropic ships a version we haven't hardcoded — the opposite of the current
failure profile.

- *Ordering note:* `fable` / `mythos` share no substring with `opus` / `sonnet` /
  `haiku`, so the new arms can go anywhere in the `if` chain without shadowing.
  Placed after `haiku` to keep the cheapest→most-expensive reading order broken
  only once.

**D3 — Do NOT model Sonnet 5's introductory pricing. (Confirmed by the user
2026-07-24: "keep sonnet at normal price.")** Sonnet 5 lists at $3/$15 but
carries an introductory $2/$10 per MTok through **2026-08-31** — currently
active, and Sonnet 5 is 33,484 events, so today's Sonnet 5 cost is overstated by
~50%. We still price it at list, because:
  1. Modelling it requires date-windowed rows — `cost(model, usage)` becomes
     `cost(model, usage, timestamp)`, which ripples into the recompute migration
     and doubles the correctness surface.
  2. The window closes in ~5 weeks, after which list price is right forever; the
     machinery would be dead weight almost immediately.
  3. TokenWatch cost is an **estimate** for subscription users who don't pay per
     token. List price is the stable, defensible baseline; a promo rate that
     silently expires is a worse invariant than a consistently-list number.
- *Alternative available if the user disagrees:* add `valid_from`/`valid_until`
  to `PriceRow` and select by event `timestamp`. Deferrable without rework — the
  recompute migration means switching later still repairs history.

**D6 — Claude Opus 5 needs no price row; it is already correct.** Verified against
the live pricing page on 2026-07-24: **Claude Opus 5 is $5 / $25 per MTok, with
$6.25 cache write and $0.50 cache read — byte-identical to Opus 4.5 through 4.8.**
The 35 stored `claude-opus-5` events therefore already price correctly through the
existing `OPUS` row (substring `opus` hits), and `claude-opus-5` is already
asserted in `test_all_model_variants_resolve`. Adding a dedicated `OPUS_5` row
would duplicate four identical numbers and create two places to edit when the Opus
tier reprices.

This is D2 paying off: the version-agnostic matcher absorbed a whole new
generation with zero code change. Only a new *family name* (Fable/Mythos) needed
work.

- *Known limitation, deliberately not addressed:* the family matcher does misprice
  two **retired/deprecated** rows — Opus 4.1 and Opus 4 are $15/$75 (matcher says
  $5/$25, a 3× understatement) and Haiku 3.5 is $0.80/$4 (matcher says $1/$5).
  Neither appears in the observed logs, and Claude Code does not emit them.
  Correcting them means per-version rows for dead models, which trades a real
  simplicity win for zero practical accuracy. Revisit only if such events appear.

**D4 — Repair history with a `v7` migration that reruns `recompute_costs`.**
Exactly the `v6` precedent, and the `usage-ingestion` spec already *requires*
this ("Reconciliación del costo persistido ante cambios en la tabla de
precios"): a table change must repair stored events, not just new ingests.
`apply_v7` is a one-line call; `recompute_costs` opens its own transaction, so
the `migrate` block must **not** wrap it (same note as `v4`/`v6`).

- *Alternative rejected — clear `ingest_files` to force re-ingest:* slow, and
  broken by design if the source JSONL has been rotated or deleted; SQLite is the
  source of truth precisely so cost survives log deletion.

**D5 — No new observability surface for unknown models.** The real lesson is that
`cost = 0` + a warn log is a silent failure, but the fix for that is a guard
test (extend `test_all_model_variants_resolve` with `claude-fable-5` and
`claude-mythos-5`), not runtime UI. A "model unpriced" badge in the app is a
separate, larger change — it needs a place to live in the dashboard and a
dismissal story. Out of scope here; worth its own proposal.

## Risks / Trade-offs

- **[Historical cost totals jump on the launch that crosses v6→v7]** → Intended:
  244 Fable events go from $0 to their real cost. Users comparing to a
  screenshot will see a step change. The `usage-ingestion` reconciliation
  requirement already documents this as the intended contract.
- **[Sonnet 5 stays ~50% over-reported until 2026-08-31]** → Accepted per D3, and
  it self-corrects when the intro window closes. Documented as an open question
  so it is a decision, not an oversight.
- **[Fable list price could differ from what a given org is actually billed]** →
  True of every row in the table (enterprise discounts, Bedrock/Vertex markups).
  Cost has always been a list-price estimate; unchanged by this proposal.
- **[Another new family ships and silently costs 0 again]** → Mitigated, not
  eliminated, by the guard test — it only covers families we already know about.
  D5 notes the durable fix (a visible unpriced-model signal) as follow-up work.
- **[Substring `fable`/`mythos` false-positives]** → A non-Anthropic model whose
  id contains "fable" would price at Opus-plus rates. No such model exists in
  the observed data, and the same theoretical risk already applies to
  `opus`/`sonnet`/`haiku`.

## Migration Plan

1. Ship the `FABLE` row + `price_row` arms in `pricing.rs`, with the module doc's
   verification date and the pricing-source test updated.
2. Ship `apply_v7` + the `version < 7` block in `migrate`; bump the two
   `schema_version` assertions (`db/mod.rs` `test_schema_creation`,
   `budgets/mod.rs:684`) to `"7"`.
3. On next launch, `open_default` → `migrate` sees `6 < 7`, reruns
   `recompute_costs`, sets version `7`. The launch refresh emits `usage-updated`
   and the charts re-read corrected cost.
4. **Rollback**: no data rollback needed — `cost` is re-derivable from stored
   token counts. Reverting the binary leaves the DB at `v7`; `migrate` only
   applies *missing lower* versions, so an older binary opens it safely (it would
   just re-price Fable at 0 for any newly ingested events).

## Open Questions

- **Sonnet 5 introductory pricing** — resolved 2026-07-24: the user chose to keep
  Sonnet at list price ($3/$15). No date-windowed pricing in this change (D3).
- **Claude Opus 5 pricing** — resolved 2026-07-24: verified at $5/$25, identical to
  Opus 4.x, already handled by the existing `OPUS` row. No code change (D6).
