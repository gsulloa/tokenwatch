import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import type { SeriesResponse } from "./types";

// Mock Tauri APIs before importing the hook
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Import mocked modules AFTER the vi.mock calls
const MOCK_SERIES_RESPONSE: SeriesResponse = {
  bucket: "day",
  metric: "tokens",
  buckets: ["2026-07-01", "2026-07-02"],
  series: [{ name: "claude-opus-4-8", points: [100, 200] }],
};

describe("useUsageSeries", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns loading=true initially then resolves data", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(MOCK_SERIES_RESPONSE); // query_series
    vi.mocked(invoke).mockResolvedValueOnce(null); // usage_meta

    const { useUsageSeries } = await import("./useUsageSeries");
    const { result } = renderHook(() =>
      useUsageSeries({ bucket: "day", metric: "tokens", seriesBy: "model" }),
    );

    expect(result.current.loading).toBe(true);

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data).toEqual(MOCK_SERIES_RESPONSE);
    expect(result.current.error).toBeNull();
  });

  it("exposes a refresh function that re-invokes the command", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue(null);

    const { useUsageSeries } = await import("./useUsageSeries");
    const { result } = renderHook(() =>
      useUsageSeries({ bucket: "day", metric: "tokens", seriesBy: "model" }),
    );

    await waitFor(() => expect(result.current.loading).toBe(false));

    const callCountBefore = vi.mocked(invoke).mock.calls.length;
    result.current.refresh();
    await waitFor(() =>
      expect(vi.mocked(invoke).mock.calls.length).toBeGreaterThan(
        callCountBefore,
      ),
    );
  });
});
