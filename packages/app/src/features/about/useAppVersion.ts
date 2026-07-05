import { useState, useEffect } from "react";
import { getAppVersion } from "./appVersionClient";

export interface UseAppVersionResult {
  /** App version string (e.g. "0.1.0"), or null outside Tauri. */
  version: string | null;
  /** True when running inside Tauri (version was successfully resolved). */
  isTauri: boolean;
}

/**
 * Hook that fetches the running app version once on mount.
 * Returns { version: null, isTauri: false } outside Tauri environments.
 */
export function useAppVersion(): UseAppVersionResult {
  const [version, setVersion] = useState<string | null>(null);
  const [isTauri, setIsTauri] = useState(false);

  useEffect(() => {
    void getAppVersion().then((v) => {
      if (v !== null) {
        setVersion(v);
        setIsTauri(true);
      }
    });
  }, []);

  return { version, isTauri };
}
