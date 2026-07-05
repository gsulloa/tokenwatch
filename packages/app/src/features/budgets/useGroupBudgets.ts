import { useState, useEffect, useCallback } from "react";
import type { GroupBudgetsSnapshot, RawGroupBudgetsSnapshot } from "./types";
import { normalizeGroupBudgetsSnapshot } from "./types";

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

interface UseGroupBudgetsResult {
  snapshot: GroupBudgetsSnapshot | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Hook that fetches group budget data via `query_group_budgets` and subscribes to
 * `limits-updated` and `usage-updated` Tauri events. Exposes a normalized snapshot.
 * Mirrors the useLimits.ts pattern exactly.
 */
export function useGroupBudgets(): UseGroupBudgetsResult {
  const [snapshot, setSnapshot] = useState<GroupBudgetsSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const raw = await safeTauriInvoke<RawGroupBudgetsSnapshot>("query_group_budgets");
      if (raw !== null) {
        setSnapshot(normalizeGroupBudgetsSnapshot(raw));
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  }, []);

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

  // Subscribe to usage-updated events
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    void safeTauriListen("usage-updated", () => {
      void fetchData();
    }).then((unlisten) => {
      unlistenFn = unlisten;
    });

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [fetchData]);

  return { snapshot, loading, error, refresh: fetchData };
}
