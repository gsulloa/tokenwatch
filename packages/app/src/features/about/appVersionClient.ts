/**
 * Thin isolated wrapper around `@tauri-apps/api/app::getVersion()`.
 * Mirrors the dynamic-import + try/catch guard pattern used by updaterClient:
 * degrades silently to null when running outside Tauri (browser dev, Vitest).
 */

/**
 * Returns the app version string (e.g. "0.1.0") from the Tauri runtime,
 * or null when the API is unavailable (non-Tauri environment or error).
 */
export async function getAppVersion(): Promise<string | null> {
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    return await getVersion();
  } catch {
    return null;
  }
}
