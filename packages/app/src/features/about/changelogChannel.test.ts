import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the Tauri APIs the channel dynamically imports.
const invoke = vi.fn().mockResolvedValue(undefined);
const emit = vi.fn().mockResolvedValue(undefined);
const unlisten = vi.fn();
const listen = vi.fn().mockResolvedValue(unlisten);

vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock("@tauri-apps/api/event", () => ({
  emit: (...a: unknown[]) => emit(...a),
  listen: (...a: unknown[]) => listen(...a),
}));

import { openChangelogInDashboard, onOpenChangelogRequest } from "./changelogChannel";

/** Flush pending microtasks + a macrotask so dynamic imports resolve. */
const flush = () => new Promise((r) => setTimeout(r, 0));

describe("changelogChannel", () => {
  beforeEach(() => {
    invoke.mockClear();
    emit.mockClear();
    listen.mockClear();
    unlisten.mockClear();
  });

  it("openChangelogInDashboard opens the dashboard window and emits the event", async () => {
    await openChangelogInDashboard();
    expect(invoke).toHaveBeenCalledWith("open_dashboard");
    expect(emit).toHaveBeenCalledWith("open-changelog");
  });

  it("onOpenChangelogRequest subscribes to the event and invokes the handler", async () => {
    const handler = vi.fn();
    onOpenChangelogRequest(handler);
    // Allow the async dynamic-import + listen() chain to resolve.
    await flush();
    expect(listen).toHaveBeenCalledWith("open-changelog", expect.any(Function));
    // Fire the registered callback and confirm it reaches the handler.
    const registered = listen.mock.calls[0]?.[1] as (() => void) | undefined;
    registered?.();
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("onOpenChangelogRequest returns a cleanup that unsubscribes", async () => {
    const cleanup = onOpenChangelogRequest(vi.fn());
    await flush();
    cleanup();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });
});
