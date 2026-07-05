/**
 * Cross-window channel for opening the full changelog in the dashboard (main)
 * window.
 *
 * The popover is a small, fixed 360×480 window — the full changelog modal does
 * not fit there. So the changelog lives in the resizable main window instead.
 * The main window's React app is always mounted (it starts hidden and is merely
 * shown/focused by the `open_dashboard` command), so its `open-changelog`
 * listener is active from startup and receives this event reliably — no
 * emit/listen race.
 *
 * Both functions degrade to a no-op outside Tauri (browser dev, Vitest),
 * mirroring the `updaterClient` / `appVersionClient` guard pattern.
 */

const OPEN_CHANGELOG_EVENT = "open-changelog";

/**
 * From the popover: reveal the dashboard window and ask it to open the full
 * changelog. Safe to call outside Tauri (no-op).
 */
export async function openChangelogInDashboard(): Promise<void> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("open_dashboard");
  } catch {
    // Outside Tauri or command unavailable — ignore.
  }
  try {
    const { emit } = await import("@tauri-apps/api/event");
    await emit(OPEN_CHANGELOG_EVENT);
  } catch {
    // Outside Tauri — ignore.
  }
}

/**
 * In the dashboard: subscribe to changelog-open requests coming from the
 * popover. Returns a cleanup function to unsubscribe. No-op outside Tauri.
 */
export function onOpenChangelogRequest(handler: () => void): () => void {
  let unlisten: (() => void) | undefined;
  let cancelled = false;

  void (async () => {
    try {
      const { listen } = await import("@tauri-apps/api/event");
      const un = await listen(OPEN_CHANGELOG_EVENT, () => handler());
      if (cancelled) un();
      else unlisten = un;
    } catch {
      // Outside Tauri — no listener.
    }
  })();

  return () => {
    cancelled = true;
    if (unlisten) unlisten();
  };
}
