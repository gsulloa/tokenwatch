/**
 * Budget basis type. `share` = % of local window cost; `usd` = absolute USD.
 * null means no cap.
 */
export type BudgetBasis = "share" | "usd";

/**
 * A project group as persisted in `project_groups`.
 * Fields are camelCase (normalized from Rust serde snake_case by hooks).
 */
export interface Group {
  id: number;
  name: string;
  budgetBasis: BudgetBasis | null;
  budgetValue: number | null;
}

/**
 * A group with its assigned project names.
 */
export interface GroupWithMembers {
  group: Group;
  members: string[];
}

/**
 * A single row in the group budgets snapshot.
 * `groupId` is null for the implicit "otros" bucket.
 */
export interface GroupBudgetRow {
  groupId: number | null;
  name: string;
  budgetBasis: BudgetBasis | null;
  budgetValue: number | null;
  windowCostUsd: number;
  /** Pure local ratio: windowCostUsd / totalWindowCost × 100. NOT multiplied by session.utilization. */
  localCostSharePct: number;
  /**
   * Session-weighted estimate: localCostSharePct × session.utilization / 100.
   * Only present when origin = "session". null in rolling mode.
   * Labeled "est." in the UI. Summed across groups ≈ session.utilization.
   */
  sessionWeightedPct: number | null;
  /** share → sessionWeightedPct (session, est.); usd → windowCostUsd; null → no cap. */
  measuredValue: number | null;
}

/**
 * Snapshot returned by `query_group_budgets`.
 * `origin` = "session" when anchored to `session.resets_at`, else "rolling".
 */
export interface GroupBudgetsSnapshot {
  rows: GroupBudgetRow[];
  windowStart: string;
  origin: "session" | "rolling";
}

// ── Raw types (snake_case from Tauri serde) ───────────────────────────────────

export interface RawGroup {
  id: number;
  name: string;
  budget_basis?: BudgetBasis | null;
  budgetBasis?: BudgetBasis | null;
  budget_value?: number | null;
  budgetValue?: number | null;
}

export interface RawGroupWithMembers {
  group: RawGroup;
  members: string[];
}

export interface RawGroupBudgetRow {
  group_id?: number | null;
  groupId?: number | null;
  name: string;
  budget_basis?: BudgetBasis | null;
  budgetBasis?: BudgetBasis | null;
  budget_value?: number | null;
  budgetValue?: number | null;
  window_cost_usd?: number;
  windowCostUsd?: number;
  local_cost_share_pct?: number;
  localCostSharePct?: number;
  session_weighted_pct?: number | null;
  sessionWeightedPct?: number | null;
  measured_value?: number | null;
  measuredValue?: number | null;
}

export interface RawGroupBudgetsSnapshot {
  rows: RawGroupBudgetRow[];
  window_start?: string;
  windowStart?: string;
  origin: string;
}

// ── Normalizers ───────────────────────────────────────────────────────────────

export function normalizeGroup(raw: RawGroup): Group {
  return {
    id: raw.id,
    name: raw.name,
    budgetBasis: raw.budgetBasis ?? raw.budget_basis ?? null,
    budgetValue: raw.budgetValue ?? raw.budget_value ?? null,
  };
}

export function normalizeGroupWithMembers(raw: RawGroupWithMembers): GroupWithMembers {
  return {
    group: normalizeGroup(raw.group),
    members: raw.members,
  };
}

export function normalizeGroupBudgetRow(raw: RawGroupBudgetRow): GroupBudgetRow {
  return {
    groupId: raw.groupId ?? raw.group_id ?? null,
    name: raw.name,
    budgetBasis: raw.budgetBasis ?? raw.budget_basis ?? null,
    budgetValue: raw.budgetValue ?? raw.budget_value ?? null,
    windowCostUsd: raw.windowCostUsd ?? raw.window_cost_usd ?? 0,
    localCostSharePct: raw.localCostSharePct ?? raw.local_cost_share_pct ?? 0,
    sessionWeightedPct: raw.sessionWeightedPct ?? raw.session_weighted_pct ?? null,
    measuredValue: raw.measuredValue ?? raw.measured_value ?? null,
  };
}

export function normalizeGroupBudgetsSnapshot(raw: RawGroupBudgetsSnapshot): GroupBudgetsSnapshot {
  return {
    rows: raw.rows.map(normalizeGroupBudgetRow),
    windowStart: raw.windowStart ?? raw.window_start ?? "",
    origin: raw.origin === "session" ? "session" : "rolling",
  };
}
