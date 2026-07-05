import { useState, useEffect, useCallback, useRef } from "react";
import type { Update } from "@tauri-apps/plugin-updater";
import { checkForUpdate, relaunchApp as clientRelaunch } from "./updaterClient";
import type { UpdateStatus, UseAppUpdateResult } from "./types";

/** Re-check interval: every 6 hours. */
const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

/**
 * Hook that manages the full app-update lifecycle.
 *
 * - Auto-checks once on mount (background, silent on failure).
 * - Re-checks periodically every CHECK_INTERVAL_MS.
 * - Exposes manual `checkNow()`, `installNow()`, and `relaunchApp()` actions.
 * - Outside Tauri (dev browser / tests) stays `idle` without throwing.
 */
export function useAppUpdate(): UseAppUpdateResult {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [version, setVersion] = useState<string | null>(null);
  const [notes, setNotes] = useState<string | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Hold the Update object returned by the plugin for installNow().
  const pendingUpdate = useRef<Update | null>(null);
  // Guard against concurrent install calls.
  const isInstalling = useRef(false);

  /**
   * Internal check routine.
   * `isManual` controls whether a failure surfaces an error in the UI.
   */
  const runCheck = useCallback(async (isManual: boolean) => {
    setStatus("checking");
    if (isManual) {
      setError(null);
    }

    try {
      const update = await checkForUpdate();

      if (update) {
        pendingUpdate.current = update;
        setVersion(update.version ?? null);
        setNotes(update.body ?? null);
        setStatus("available");
      } else {
        // No update available (or non-Tauri env returned null).
        // Only reset to idle if we were checking — don't override ready/downloading.
        setStatus((prev) =>
          prev === "checking" ? "idle" : prev,
        );
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : "Error desconocido";
      console.error("[useAppUpdate] check failed:", message);

      if (isManual) {
        setError(message);
        setStatus("error");
      } else {
        // Background failure: log only, stay idle.
        setStatus("idle");
      }
    }
  }, []);

  // Auto-check once on mount (background).
  useEffect(() => {
    void runCheck(false);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Periodic re-check.
  useEffect(() => {
    const id = setInterval(() => {
      void runCheck(false);
    }, CHECK_INTERVAL_MS);
    return () => clearInterval(id);
  }, [runCheck]);

  /** Force an immediate update check (manual — errors surface in UI). */
  const checkNow = useCallback(() => {
    void runCheck(true);
  }, [runCheck]);

  /** Download and install the pending update. */
  const installNow = useCallback(async () => {
    if (isInstalling.current || !pendingUpdate.current) return;
    isInstalling.current = true;
    setStatus("downloading");
    setProgress(0);

    // Track total size from the Started event so we can compute progress %.
    let totalBytes: number | null = null;
    let downloadedBytes = 0;

    try {
      await pendingUpdate.current.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? null;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          if (totalBytes != null && totalBytes > 0) {
            const pct = Math.min(
              100,
              Math.round((downloadedBytes / totalBytes) * 100),
            );
            setProgress(pct);
          }
        }
      });
      setProgress(100);
      setStatus("ready");
    } catch (err) {
      const message = err instanceof Error ? err.message : "Error al instalar";
      console.error("[useAppUpdate] install failed:", message);
      setError(message);
      setStatus("error");
    } finally {
      isInstalling.current = false;
    }
  }, []);

  /** Relaunch the app to apply the installed update (explicit user action). */
  const relaunchApp = useCallback(() => {
    void clientRelaunch();
  }, []);

  return {
    status,
    version,
    notes,
    progress,
    error,
    checkNow,
    installNow: () => void installNow(),
    relaunchApp,
  };
}
