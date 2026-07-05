/**
 * State machine for the app update lifecycle.
 * idle      → no update info yet (initial state, or up-to-date after check)
 * checking  → actively polling the update endpoint
 * available → a newer version was found, ready to install
 * downloading → installing the update (download + patch apply)
 * ready     → update applied, waiting for user to relaunch
 * error     → last check or install failed (only surfaced on manual action)
 */
export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error";

/** Result exposed by `useAppUpdate`. */
export interface UseAppUpdateResult {
  status: UpdateStatus;
  /** Available version string (e.g. "1.2.3"), or null when idle / error. */
  version: string | null;
  /** Release notes for the available version, or null. */
  notes: string | null;
  /** Download progress 0–100, only meaningful in `downloading` state. */
  progress: number | null;
  /** Error message, only populated on manual-check failures. */
  error: string | null;
  /** Force an immediate update check (manual). */
  checkNow: () => void;
  /** Start downloading and installing the available update. */
  installNow: () => void;
  /** Relaunch the app to apply an installed update (`ready` state). */
  relaunchApp: () => void;
}
