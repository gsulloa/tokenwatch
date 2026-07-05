import { useState, useEffect } from "react";
import { useAppVersion } from "@/features/about/useAppVersion";
import { extractVersionSection } from "./changelogParser";
import { getLastSeenVersion, setLastSeenVersion } from "./lastSeenVersion";
import changelogRaw from "@/generated/changelog.md?raw";

export interface UseWhatsNewResult {
  /** Whether to show the What's New modal. */
  show: boolean;
  /** Current app version, or null outside Tauri. */
  version: string | null;
  /** Extracted changelog section for the current version, or null. */
  versionSection: string | null;
  /** Call to dismiss the modal; persists current version as last-seen. */
  dismiss: () => void;
}

/**
 * Hook that compares the running version to the last-seen version and decides
 * whether to show the What's New modal.
 *
 * Logic:
 * - Non-Tauri / version null → show=false, no-op.
 * - No last-seen stored (first install) → mark current as seen, show=false.
 * - current === last-seen → show=false.
 * - current !== last-seen:
 *     - Extract changelog section for current version.
 *     - If section found → show=true.
 *     - If no section   → show=false but STILL mark current as seen (avoid
 *       re-checking on every launch).
 *
 * dismiss() persists the current version as last-seen and hides the modal.
 *
 * The changelog markdown and version are injectable for testability via the
 * optional `_changelog` and `_useVersionHook` parameters.
 */
export function useWhatsNew(
  _changelog: string = changelogRaw,
  _useVersionHook: () => { version: string | null } = useAppVersion,
): UseWhatsNewResult {
  const { version } = _useVersionHook();
  const [show, setShow] = useState(false);
  const [versionSection, setVersionSection] = useState<string | null>(null);

  useEffect(() => {
    // Can't do anything without a real version (non-Tauri environment).
    if (version === null) return;

    const lastSeen = getLastSeenVersion();

    if (lastSeen === null) {
      // First install: persist current, do NOT show modal.
      setLastSeenVersion(version);
      return;
    }

    if (lastSeen === version) {
      // Already seen this version.
      return;
    }

    // Version changed — extract changelog section.
    const section = extractVersionSection(_changelog, version);

    if (section !== null) {
      setVersionSection(section);
      setShow(true);
    } else {
      // No section found: mark as seen anyway so we don't retry every launch.
      setLastSeenVersion(version);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [version]);

  const dismiss = () => {
    if (version !== null) {
      setLastSeenVersion(version);
    }
    setShow(false);
  };

  return { show, version, versionSection, dismiss };
}
