import { useState, useEffect, useCallback, useRef } from "react";
import type { LimitsSnapshot, LimitsWindow } from "./types";

/**
 * Safely invoke a Tauri command. Returns null in non-Tauri (e.g. jsdom/test) environments.
 */
async function safeTauriInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(command, args);
  } catch {
    return null;
  }
}

/**
 * Safely listen to a Tauri event. Returns an unlisten function (no-op outside Tauri).
 */
async function safeTauriListen(
  event: string,
  handler: () => void,
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    return await listen(event, handler);
  } catch {
    return () => {};
  }
}

/**
 * Raw window shape from backend — serde may use snake_case or camelCase for resets_at.
 */
interface RawWindow {
  label?: string | null;
  utilization: number;
  resets_at?: string;
  resetsAt?: string;
}

/**
 * Normalize a raw window from the backend into the clean LimitsWindow shape.
 * Handles both `resets_at` (snake_case serde) and `resetsAt` (camelCase) field names.
 */
function normalizeWindow(raw: RawWindow): LimitsWindow {
  return {
    label: raw.label ?? null,
    utilization: raw.utilization,
    resetsAt: raw.resetsAt ?? raw.resets_at ?? "",
  };
}

/**
 * Raw snapshot shape from backend — mirrors Rust serde output.
 * weekly_by_model may arrive as weeklyByModel depending on serde rename.
 */
interface RawSnapshot {
  session?: RawWindow | null;
  weekly?: RawWindow | null;
  weekly_by_model?: RawWindow[];
  weeklyByModel?: RawWindow[];
  fetched_at?: string;
  fetchedAt?: string;
  status?: { kind: string; reason?: string };
}

/**
 * Normalize the raw backend snapshot into the clean LimitsSnapshot shape.
 */
function normalizeSnapshot(raw: RawSnapshot): LimitsSnapshot {
  const rawStatus = raw.status ?? { kind: "unavailable", reason: "parse" };
  const status =
    rawStatus.kind === "ok"
      ? ({ kind: "ok" } as const)
      : ({
          kind: "unavailable",
          reason: (rawStatus.reason ?? "parse") as
            | "not_signed_in"
            | "keychain_denied"
            | "expired"
            | "rate_limited"
            | "network"
            | "http"
            | "parse",
        } as const);

  const weeklyByModelRaw =
    raw.weeklyByModel ?? raw.weekly_by_model ?? [];

  return {
    session: raw.session ? normalizeWindow(raw.session) : null,
    weekly: raw.weekly ? normalizeWindow(raw.weekly) : null,
    weeklyByModel: weeklyByModelRaw.map(normalizeWindow),
    fetchedAt: raw.fetchedAt ?? raw.fetched_at ?? "",
    status,
  };
}

/** Minimum ms between on-open refreshes to avoid hammering the API on rapid toggles. */
const MIN_REFRESH_INTERVAL_MS = 10_000;

interface UseLimitsResult {
  snapshot: LimitsSnapshot | null;
  loading: boolean;
  error: string | null;
  /** Force an unconditional fetch regardless of the last fetch time. */
  refresh: () => void;
  /** Fetch only if more than MIN_REFRESH_INTERVAL_MS has elapsed since the last successful fetch. */
  refreshIfStale: () => void;
}

/**
 * Hook that fetches Claude limits via `query_limits` and subscribes to
 * `limits-updated` Tauri events. Exposes a normalized LimitsSnapshot.
 */
export function useLimits(): UseLimitsResult {
  const [snapshot, setSnapshot] = useState<LimitsSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /** Timestamp (ms) of the last SUCCESSFUL fetch. 0 = never fetched. */
  const lastFetchAtRef = useRef<number>(0);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const raw = await safeTauriInvoke<RawSnapshot>("query_limits");
      if (raw !== null) {
        setSnapshot(normalizeSnapshot(raw));
        lastFetchAtRef.current = Date.now();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  }, []);

  /**
   * Throttled refresh: skips the fetch when a successful fetch completed within
   * the last MIN_REFRESH_INTERVAL_MS to avoid redundant API calls on rapid
   * popover open/close toggles.
   */
  const refreshIfStale = useCallback(() => {
    if (Date.now() - lastFetchAtRef.current < MIN_REFRESH_INTERVAL_MS) {
      return;
    }
    void fetchData();
  }, [fetchData]);

  // Fetch on mount
  useEffect(() => {
    void fetchData();
  }, [fetchData]);

  // Subscribe to limits-updated events
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    void safeTauriListen("limits-updated", () => {
      void fetchData();
    }).then((unlisten) => {
      unlistenFn = unlisten;
    });

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [fetchData]);

  return { snapshot, loading, error, refresh: fetchData, refreshIfStale };
}
