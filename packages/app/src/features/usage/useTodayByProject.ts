import { useState, useEffect, useCallback } from "react";
import type { TodayByProject } from "@/features/limits/types";

/**
 * Safely invoke a Tauri command. Returns null in non-Tauri environments.
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

interface UseTodayByProjectResult {
  data: TodayByProject | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Hook that fetches today's token usage by project via `query_today_by_project`
 * and re-fetches whenever the `usage-updated` event fires.
 */
export function useTodayByProject(): UseTodayByProjectResult {
  const [data, setData] = useState<TodayByProject | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await safeTauriInvoke<TodayByProject>(
        "query_today_by_project",
      );
      if (result !== null) {
        setData(result);
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

  return { data, loading, error, refresh: fetchData };
}
