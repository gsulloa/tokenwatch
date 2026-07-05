import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

// Mock the updaterClient module so tests never touch real Tauri plugins.
// Must be declared before any imports of modules that depend on updaterClient.
vi.mock("./updaterClient", () => ({
  checkForUpdate: vi.fn(),
  relaunchApp: vi.fn(),
}));

// Import hook and mocks AFTER vi.mock declarations.
import { useAppUpdate } from "./useAppUpdate";
import { checkForUpdate, relaunchApp } from "./updaterClient";

const mockCheckForUpdate = vi.mocked(checkForUpdate);
const mockRelaunchApp = vi.mocked(relaunchApp);

describe("useAppUpdate", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it("status becomes available with version when a new update is found", async () => {
    const fakeUpdate = {
      version: "1.2.3",
      body: "Release notes here",
      downloadAndInstall: vi.fn(),
    };
    mockCheckForUpdate.mockResolvedValueOnce(fakeUpdate as never);

    const { result } = renderHook(() => useAppUpdate());

    await waitFor(() => {
      expect(result.current.status).toBe("available");
    });

    expect(result.current.version).toBe("1.2.3");
    expect(result.current.notes).toBe("Release notes here");
    expect(result.current.error).toBeNull();
  });

  it("stays idle in a non-Tauri env (checkForUpdate returns null)", async () => {
    mockCheckForUpdate.mockResolvedValueOnce(null);

    const { result } = renderHook(() => useAppUpdate());

    await waitFor(() => {
      // After the (async) auto-check resolves with null, status must be idle.
      expect(mockCheckForUpdate).toHaveBeenCalledTimes(1);
    });

    // Give React one more tick to settle state.
    await waitFor(() => {
      expect(result.current.status).toBe("idle");
    });

    expect(result.current.error).toBeNull();
    expect(result.current.version).toBeNull();
  });

  it("installNow transitions downloading → ready and relaunch is callable", async () => {
    let resolveInstall!: () => void;
    const installPromise = new Promise<void>((res) => {
      resolveInstall = res;
    });

    const fakeUpdate = {
      version: "2.0.0",
      body: null,
      downloadAndInstall: vi.fn(async (_onEvent: unknown) => {
        await installPromise;
      }),
    };
    mockCheckForUpdate.mockResolvedValueOnce(fakeUpdate as never);
    mockRelaunchApp.mockResolvedValue(undefined);

    const { result } = renderHook(() => useAppUpdate());

    // Wait until available
    await waitFor(() => {
      expect(result.current.status).toBe("available");
    });

    // Trigger install
    act(() => {
      result.current.installNow();
    });

    await waitFor(() => {
      expect(result.current.status).toBe("downloading");
    });

    // Resolve the install
    resolveInstall();

    await waitFor(() => {
      expect(result.current.status).toBe("ready");
    });

    // Relaunch should be callable without throwing
    act(() => {
      result.current.relaunchApp();
    });

    await waitFor(() => {
      expect(mockRelaunchApp).toHaveBeenCalledTimes(1);
    });
  });

  it("background auto-check failure → error logged, status stays idle; manual checkNow surfaces error", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    // First call (auto, background): throws
    mockCheckForUpdate.mockRejectedValueOnce(new Error("Network error"));

    const { result } = renderHook(() => useAppUpdate());

    // Background failure → stays idle, no error surfaced
    await waitFor(() => {
      expect(mockCheckForUpdate).toHaveBeenCalledTimes(1);
    });

    await waitFor(() => {
      expect(result.current.status).toBe("idle");
    });

    expect(result.current.error).toBeNull();
    expect(consoleSpy).toHaveBeenCalledWith(
      expect.stringContaining("[useAppUpdate]"),
      expect.any(String),
    );

    // Manual check: also fails → error IS surfaced
    mockCheckForUpdate.mockRejectedValueOnce(new Error("Manual error"));

    act(() => {
      result.current.checkNow();
    });

    await waitFor(() => {
      expect(result.current.status).toBe("error");
    });

    expect(result.current.error).toBe("Manual error");

    consoleSpy.mockRestore();
  });
});
