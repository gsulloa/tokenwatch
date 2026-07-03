import { useState, useEffect, useCallback, useRef } from "react";
import type { SeriesQuery, SeriesResponse, UsageMeta } from "./types";

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
    // Not in a Tauri environment or command failed
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

interface UseUsageSeriesResult {
  data: SeriesResponse | null;
  meta: UsageMeta | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Hook that queries the Tauri backend for time-series usage data.
 * Subscribes to the `usage-updated` event and refetches automatically.
 */
export function useUsageSeries(query: SeriesQuery): UseUsageSeriesResult {
  const [data, setData] = useState<SeriesResponse | null>(null);
  const [meta, setMeta] = useState<UsageMeta | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Use a ref to avoid stale closure in the event listener
  const queryRef = useRef(query);
  queryRef.current = query;

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [seriesResult, metaResult] = await Promise.all([
        safeTauriInvoke<SeriesResponse>("query_series", {
          params: queryRef.current,
        }),
        safeTauriInvoke<UsageMeta>("usage_meta"),
      ]);
      setData(seriesResult);
      setMeta(metaResult);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  }, []);

  // Fetch on mount and whenever the query changes
  useEffect(() => {
    void fetchData();
  }, [fetchData, query.bucket, query.metric, query.seriesBy, query.since, query.until]);

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

  return { data, meta, loading, error, refresh: fetchData };
}
