/**
 * Persistence helpers for the "last seen version" used by the What's New feature.
 * Uses localStorage under the key `tokenwatch.lastSeenVersion`.
 * All calls are wrapped in try/catch to silently degrade when localStorage is
 * unavailable (e.g. private browsing, non-browser environments).
 */

const STORAGE_KEY = "tokenwatch.lastSeenVersion";

/**
 * Returns the last version the user was shown What's New for, or null if
 * no value has been stored yet (first install or cleared storage).
 */
export function getLastSeenVersion(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

/**
 * Persists the given version as the last seen version.
 */
export function setLastSeenVersion(v: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, v);
  } catch {
    // Storage unavailable — silently ignore.
  }
}
