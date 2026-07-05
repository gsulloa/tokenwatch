/**
 * Thin isolated wrapper around `@tauri-apps/plugin-updater` and
 * `@tauri-apps/plugin-process`. Mirrors the `safeTauriInvoke` pattern:
 * dynamic imports inside try/catch so the module degrades silently when
 * running outside Tauri (browser dev, Vitest).
 *
 * Keeping the plugin access in one place makes it trivial to mock in tests
 * via `vi.mock("./updaterClient")`.
 */

import type { Update } from "@tauri-apps/plugin-updater";

/**
 * Check for an available update via the configured updater endpoint.
 * Returns the `Update` object if a newer version exists, or `null` when
 * the app is up-to-date or the call fails (non-Tauri env, network error, etc.).
 */
export async function checkForUpdate(): Promise<Update | null> {
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    return await check();
  } catch {
    return null;
  }
}

/**
 * Relaunch the app to apply an installed update.
 * No-op (and throw-safe) outside a real Tauri environment.
 */
export async function relaunchApp(): Promise<void> {
  try {
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  } catch {
    // Outside Tauri or plugin not available — silently ignore.
  }
}
