# /autoplan Review — project-budgets

Branch: gsulloa/token-budgets-por-grupo · Base: dev · Reviewers: Claude subagent + Codex (codex-cli 0.139.0)

## Phase 1 — CEO Review (Strategy & Scope)

### CEO Dual Voices — Consensus Table
```
═══════════════════════════════════════════════════════════════
  Dimension                              Claude  Codex  Consensus
  ──────────────────────────────────────  ─────   ─────  ─────────
  1. Premises valid?                       NO      NO     CONFIRMED (invalid)
  2. Right problem to solve?               NO      NO     CONFIRMED (reframe)
  3. Scope calibration correct?           partial partial CONFIRMED (alt dismissed)
  4. Alternatives sufficiently explored?   NO      NO     CONFIRMED
  5. Competitive/market risks covered?     NO      NO     CONFIRMED (differentiates on a liability)
  6. 6-month trajectory sound?             NO      NO     CONFIRMED (trust regret)
═══════════════════════════════════════════════════════════════
```
Both voices reached the same verdict independently: **REFRAME**. The scaffolding
(groups, schema, dashboard editor, popover section) is sound; the load-bearing
idea (splitting Claude's opaque global session-% by local retail-USD cost-share
and alerting on it) is an unvalidated proxy presented as a measurement.

### Critical findings (both voices, CONFIRMED)
- **C1 — Attribution invalid.** `session.utilization` is an opaque global number; `pricing.rs` is retail API list price, not Anthropic's subscription session unit. "The absolute cap cancels algebraically" only holds if a single stable USD→session exchange rate exists. No evidence it does.
- **C2 — Cache reads break it.** `pricing.rs` prices cache reads at 0.1x. Cache-heavy work (large repos, the median Claude Code workload) has tiny USD but big session impact → the design systematically under-attributes session-% to exactly those groups.
- **H3 — Subscription vs retail divergence.** User is preserving a subscription session, not managing API spend. No shared denominator.
- **H4 — Absolute USD/token caps dismissed too fast.** Local USD/token caps are fully accurate and directly billing-actionable; `usage_events.cost` already exists. A wrong % is worse than an honest dollar cap.
- **H5 — Uncaptured usage contaminates.** claude.ai web / other machines move the global gauge but aren't in JSONL; the split smears that across local groups → false blame. Error is largest for the heaviest (target) user.
- **M6 — Overfits a derived metric.** Real decision is "which group is burning my session, am I about to run out." Answerable exactly from local cost-share + the existing global gauge.
- **M7 — Trust regret (6 mo).** A user reconciles a per-group % against a client bill, finds it off, posts a screenshot → damages trust in the accurate features too.
- **M8 — Gauge implies authority.** Naming it `session_pct` and drawing a gauge makes a proxy look Anthropic-backed. Rename to `local_cost_share_pct` / `estimated_session_allocation_pct` + `allocation_basis`.

### The reframe both voices recommend
Primary metric = **per-group share of local cost/tokens within the current 5h session window** (accurate, defensible), shown **alongside** the real global session gauge (existing). Budgets/alerts on an **honest basis**: absolute local USD/token cap, or share-of-local-cost cap. Demote or drop the multiplied "% of session per group" (or gate it behind a one-week instrumentation spike + explicit "estimate" label; never a silent alert trigger).

### Codebase mismatches to fix regardless of reframe (Finding 6, Claude voice)
- **No cached LimitsSnapshot exists.** `query_limits` re-fetches from network every call (`limits/mod.rs`). `query_group_budgets` on every `usage-updated` (30s) would trigger network round-trips → 429 risk. Prerequisite: add `Mutex<Option<LimitsSnapshot>>` to `AppState`, write in the poll, read in the new command.
- **Task 1.2 is dead work.** `db/mod.rs:35` already sets `PRAGMA foreign_keys=ON`.
- **`project_name` join key is fragile.** Prior migrations re-derive it; memberships silently orphan into "otros" if derivation changes. Note the coupling / consider a stabler key.

## Phase 2 — Design Review (Claude subagent + Codex)

### Design Dual Voices — Consensus (both CONFIRMED)
```
  Dimension                                Claude  Codex  Consensus
  ───────────────────────────────────────  ─────   ─────  ─────────
  Session-% vs group-share conflation risk  YES     YES    CONFIRMED (critical)
  LimitGauge reusable for share/USD cap?     NO      NO     CONFIRMED (need CapMeter)
  Rendered states specified?                 NO      NO     CONFIRMED (gaps)
  Editor fits existing UI vocabulary?        NO      NO     CONFIRMED (net-new, use .panel)
  Slop risk / token fidelity?               risk    risk   CONFIRMED
```
Design-spec completeness (data 9/10, presentation 3-4/10). Findings → auto-decided into D14–D16 (see design.md) and spec retitle + rendered-state scenarios.

## Phase 3 — Eng Review (Claude subagent + Codex)

### Eng Dual Voices — Consensus (both CONFIRMED)
```
  Dimension                                Claude  Codex  Consensus
  ───────────────────────────────────────  ─────   ─────  ─────────
  1. Architecture sound?                    mostly  mostly CONFIRMED (with fixes)
  2. Test coverage sufficient?              NO      NO     CONFIRMED (gaps → 9.1b)
  3. Timestamp comparison correct?          NO      NO     CONFIRMED (critical, D12)
  4. Migration chain correct?               NO      NO     CONFIRMED (v4 bump bug, D13)
  5. Concurrency/alert lifecycle handled?   NO      NO     CONFIRMED (D17)
  6. project_name key stable?               NO      NO     CONFIRMED (D19 ⚑ taste)
```

## Decision Audit Trail

| # | Phase | Decision | Class | Principle | Rationale |
|---|-------|----------|-------|-----------|-----------|
| 1 | CEO | Reframe to honest local-cost-share (option B) | User Challenge | — | User chose B at premise gate |
| 2 | Eng | Fix v4→v5 version-bump bug (literal "4"/"5", tx) | Mechanical | P1 | Correctness; inherited bug |
| 3 | Eng | Timestamp comparison must be fixed (Z/+00:00/millis) | Mechanical | P1 | Wrong events at window edge |
| 4 | Eng | sync Mutex for last_limits, clone+drop, no await-hold | Mechanical | P5 | Matches existing conn lock style |
| 5 | Eng | query_limits seeds cache on Ok | Mechanical | P2 | Removes 5-min startup blind spot |
| 6 | Eng | delete_group cleans budget_alert:<id> in same tx | Mechanical | P1 | No orphan/id-reuse suppression |
| 7 | Eng | compute+evaluate+persist under one conn lock | Mechanical | P1 | Avoids poll/CRUD race |
| 8 | Eng | compute materializes all defined groups first | Mechanical | P1 | Empty groups don't flicker |
| 9 | Eng | enforce share<=100 (Rust + optional CHECK) | Mechanical | P1 | Spec requires it |
| 10 | Eng | keep AUTOINCREMENT; add idx_pgm_group_id | Mechanical | P3 | Cheap defense + cascade perf |
| 11 | Design | New CapMeter; don't reuse LimitGauge for share | Mechanical | P5 | LimitGauge hardcodes %, implies ceiling |
| 12 | Design | Retitle "Costo local por grupo"; lead with $ | Mechanical | P1 | Kills conflation with global % |
| 13 | Design | Specify all rendered states (rolling/zero/loading/over-cap) | Mechanical | P1 | Sibling sections set the bar |
| 14 | Design | Editor as .panel + SegmentedControl; add list_project_names | Mechanical | P5 | No modal/tab precedent in app |
| 15 | Design | Mandate design tokens/formatCost; no emoji empty state | Mechanical | P5 | Codebase is stylistically opinionated |
| T1 | Eng | Timestamp fix = normalize string, keep index (opt A) | Taste→resolved | P3/P5 | User "sigue tu instinto" → A |
| T2 | Eng | Membership key = project_name + migration contract (opt A) | Taste→resolved | P3 | User "sigue tu instinto" → A |

## Resultado final

**APPROVED** (gate D1→A, D2→A por decisión del usuario "sigue tu instinto"). Plan reencuadrado a costo local honesto; 15 fixes auto-decididos aplicados; 2 taste decisions resueltas con la recomendación. `openspec validate --strict` ✔. Listo para `/opsx:apply project-budgets`.
