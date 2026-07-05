/**
 * A usage window from the Claude API (session 5h, weekly, or weekly-by-model).
 * `label` is the model display name for weekly_scoped windows, null otherwise.
 * `resets_at` / `resetsAt` — backend may use either; hooks normalize to `resetsAt`.
 */
export interface LimitsWindow {
  label: string | null;
  utilization: number;
  /** ISO 8601. Normalized by useLimits hook to camelCase `resetsAt`. */
  resetsAt: string;
}

/**
 * Status of the limits fetch.
 * `ok` means a fresh snapshot was retrieved.
 * Any other kind means data is unavailable and `reason` indicates why.
 */
export type LimitsStatus =
  | { kind: "ok" }
  | {
      kind: "unavailable";
      reason:
        | "not_signed_in"
        | "keychain_denied"
        | "expired"
        | "network"
        | "http"
        | "parse";
    };

/**
 * Snapshot of Claude usage limits returned by `query_limits`.
 * Also the payload of the `limits-updated` Tauri event.
 */
export interface LimitsSnapshot {
  session: LimitsWindow | null;
  weekly: LimitsWindow | null;
  weeklyByModel: LimitsWindow[];
  fetchedAt: string;
  status: LimitsStatus;
}

/**
 * A single row in the today-by-project breakdown.
 */
export interface ProjectUsageRow {
  project: string;
  tokens: number;
  pct: number;
}

/**
 * Response from `query_today_by_project`.
 */
export interface TodayByProject {
  rows: ProjectUsageRow[];
  totalTokens: number;
}
