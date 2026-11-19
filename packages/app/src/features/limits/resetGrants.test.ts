import { describe, it, expect } from "vitest";
import { formatResetGrantsLabel, summarizeResetGrants } from "./resetGrants";
import type { ResetGrantSummary } from "./types";

function g(over: Partial<ResetGrantSummary>): ResetGrantSummary {
  return {
    id: "x",
    label: "L",
    resetsLeft: 1,
    endsAt: null,
    type: null,
    usableNow: true,
    ...over,
  };
}

describe("summarizeResetGrants", () => {
  it("returns null for empty or exhausted grants", () => {
    expect(summarizeResetGrants([])).toBeNull();
    expect(summarizeResetGrants([g({ resetsLeft: 0 })])).toBeNull();
  });

  it("sums resetsLeft and picks the earliest valid endsAt", () => {
    const s = summarizeResetGrants([
      g({ resetsLeft: 1, endsAt: "2026-12-15T12:00:00Z" }),
      g({ resetsLeft: 2, endsAt: "2026-11-30T12:00:00Z" }),
      g({ resetsLeft: 1, endsAt: null }),
      g({ resetsLeft: 1, endsAt: "nope" }),
    ]);
    expect(s?.count).toBe(5);
    expect(s?.nearestEndsAt).toBe("2026-11-30T12:00:00Z");
  });
});

describe("formatResetGrantsLabel", () => {
  it("uses singular without a date", () => {
    expect(formatResetGrantsLabel({ count: 1, nearestEndsAt: null })).toBe(
      "1 reinicio disponible",
    );
  });

  it("uses plural with a date", () => {
    const label = formatResetGrantsLabel({
      count: 2,
      nearestEndsAt: "2026-11-30T12:00:00Z",
    });
    expect(label.startsWith("2 reinicios disponibles · vence el ")).toBe(true);
    expect(label).toContain("30");
  });
});
