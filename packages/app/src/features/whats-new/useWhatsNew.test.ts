import { describe, it, expect, beforeEach, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useWhatsNew } from "./useWhatsNew";
import * as lastSeenMod from "./lastSeenVersion";

// ── Fixtures ──────────────────────────────────────────────────────────────────

const CHANGELOG_WITH_SECTION = `# Changelog

## [Unreleased]

## [1.2.0] - 2024-06-01

### Added
- Great new feature

## [1.1.0] - 2024-05-01

- Older stuff
`;

const CHANGELOG_WITHOUT_SECTION = `# Changelog

## [Unreleased]

## [1.0.0] - 2024-04-01

Initial release.
`;

// Helper that returns a useAppVersion-shaped hook with a fixed version.
function makeVersionHook(version: string | null) {
  return () => ({ version, isTauri: version !== null });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("useWhatsNew", () => {
  beforeEach(() => {
    // Reset localStorage between each test.
    localStorage.clear();
    vi.restoreAllMocks();
  });

  it("does not show when version is null (non-Tauri)", () => {
    const { result } = renderHook(() =>
      useWhatsNew(CHANGELOG_WITH_SECTION, makeVersionHook(null)),
    );
    expect(result.current.show).toBe(false);
    expect(result.current.versionSection).toBeNull();
  });

  it("first install: marks version as seen, does NOT show modal", () => {
    const setSpy = vi.spyOn(lastSeenMod, "setLastSeenVersion");

    const { result } = renderHook(() =>
      useWhatsNew(CHANGELOG_WITH_SECTION, makeVersionHook("1.2.0")),
    );

    expect(result.current.show).toBe(false);
    expect(setSpy).toHaveBeenCalledWith("1.2.0");
  });

  it("already-seen version: does not show modal", () => {
    lastSeenMod.setLastSeenVersion("1.2.0");

    const { result } = renderHook(() =>
      useWhatsNew(CHANGELOG_WITH_SECTION, makeVersionHook("1.2.0")),
    );

    expect(result.current.show).toBe(false);
  });

  it("new version with matching changelog section: shows modal", () => {
    lastSeenMod.setLastSeenVersion("1.1.0");

    const { result } = renderHook(() =>
      useWhatsNew(CHANGELOG_WITH_SECTION, makeVersionHook("1.2.0")),
    );

    expect(result.current.show).toBe(true);
    expect(result.current.versionSection).toContain("Great new feature");
    expect(result.current.version).toBe("1.2.0");
  });

  it("new version but no matching changelog section: marks seen, does NOT show", () => {
    lastSeenMod.setLastSeenVersion("0.9.0");
    const setSpy = vi.spyOn(lastSeenMod, "setLastSeenVersion");

    const { result } = renderHook(() =>
      useWhatsNew(CHANGELOG_WITHOUT_SECTION, makeVersionHook("9.9.9")),
    );

    expect(result.current.show).toBe(false);
    // Should have marked 9.9.9 as seen so it doesn't retry next launch.
    expect(setSpy).toHaveBeenCalledWith("9.9.9");
  });

  it("dismiss: sets show=false and persists current version as seen", () => {
    lastSeenMod.setLastSeenVersion("1.1.0");
    const setSpy = vi.spyOn(lastSeenMod, "setLastSeenVersion");

    const { result } = renderHook(() =>
      useWhatsNew(CHANGELOG_WITH_SECTION, makeVersionHook("1.2.0")),
    );

    expect(result.current.show).toBe(true);

    act(() => {
      result.current.dismiss();
    });

    expect(result.current.show).toBe(false);
    expect(setSpy).toHaveBeenCalledWith("1.2.0");
  });

  it("dismiss is a no-op for version when version is null", () => {
    const setSpy = vi.spyOn(lastSeenMod, "setLastSeenVersion");

    const { result } = renderHook(() =>
      useWhatsNew(CHANGELOG_WITH_SECTION, makeVersionHook(null)),
    );

    act(() => {
      result.current.dismiss();
    });

    expect(result.current.show).toBe(false);
    expect(setSpy).not.toHaveBeenCalled();
  });
});
