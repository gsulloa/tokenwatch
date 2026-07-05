import { describe, it, expect } from "vitest";
import {
  resolvePreset,
  resolveCustomRange,
  isHourAllowed,
  effectiveBucket,
  HOUR_BUCKET_MAX_MS,
} from "./dateRange";

// Fixed reference time: 2026-07-04T15:30:00Z (Saturday, 15:30 UTC)
const NOW = new Date("2026-07-04T15:30:00.000Z");

describe("resolvePreset", () => {
  it("24h: since = now-24h, until = now, defaultBucket = hour", () => {
    const result = resolvePreset("24h", NOW);
    expect(result.defaultBucket).toBe("hour");
    expect(result.until).toBe("2026-07-04T15:30:00.000Z");
    // since should be exactly 24h before now
    const sinceMs = new Date(result.since!).getTime();
    const expectedMs = NOW.getTime() - 24 * 60 * 60 * 1000;
    expect(sinceMs).toBe(expectedMs);
    expect(result.since).toBe("2026-07-03T15:30:00.000Z");
  });

  it("3d: since = now-3 days, defaultBucket = hour", () => {
    const result = resolvePreset("3d", NOW);
    expect(result.defaultBucket).toBe("hour");
    expect(result.since).toBe("2026-07-01T15:30:00.000Z");
    expect(result.until).toBe("2026-07-04T15:30:00.000Z");
  });

  it("7d: since = now-7 days, defaultBucket = day", () => {
    const result = resolvePreset("7d", NOW);
    expect(result.defaultBucket).toBe("day");
    expect(result.since).toBe("2026-06-27T15:30:00.000Z");
    expect(result.until).toBe("2026-07-04T15:30:00.000Z");
  });

  it("30d: since = now-30 days, defaultBucket = day", () => {
    const result = resolvePreset("30d", NOW);
    expect(result.defaultBucket).toBe("day");
    const sinceDate = new Date(result.since!);
    // 30 days before 2026-07-04T15:30:00Z = 2026-06-04T15:30:00Z
    expect(sinceDate.toISOString()).toBe("2026-06-04T15:30:00.000Z");
    expect(result.until).toBe("2026-07-04T15:30:00.000Z");
  });

  it("month: since = start of current month local, defaultBucket = day", () => {
    const result = resolvePreset("month", NOW);
    expect(result.defaultBucket).toBe("day");
    // Start of July in local time converted to UTC
    const sinceDate = new Date(result.since!);
    // The local year/month/day should be July 1
    expect(sinceDate.getFullYear()).toBe(2026);
    expect(sinceDate.getMonth()).toBe(6); // July = 6
    expect(sinceDate.getDate()).toBe(1);
    expect(sinceDate.getHours()).toBe(0);
    expect(sinceDate.getMinutes()).toBe(0);
    expect(sinceDate.getSeconds()).toBe(0);
    expect(result.until).toBe("2026-07-04T15:30:00.000Z");
  });

  it("all: no since/until, defaultBucket = week", () => {
    const result = resolvePreset("all", NOW);
    expect(result.defaultBucket).toBe("week");
    expect(result.since).toBeUndefined();
    expect(result.until).toBeUndefined();
  });

  it("custom: returns undefined since/until with defaultBucket = day", () => {
    const result = resolvePreset("custom", NOW);
    expect(result.defaultBucket).toBe("day");
    expect(result.since).toBeUndefined();
    expect(result.until).toBeUndefined();
  });
});

describe("resolveCustomRange", () => {
  it("converts start/end local date strings to UTC ISO range", () => {
    const result = resolveCustomRange("2026-07-01", "2026-07-04", null, null);
    expect(result.defaultBucket).toBe("day");
    // Start of 2026-07-01 local = local 00:00:00 → UTC ISO
    const sinceDate = new Date(result.since!);
    expect(sinceDate.getFullYear()).toBe(2026);
    expect(sinceDate.getMonth()).toBe(6);
    expect(sinceDate.getDate()).toBe(1);
    expect(sinceDate.getHours()).toBe(0);
    expect(sinceDate.getMinutes()).toBe(0);
    // End of 2026-07-04 local = local 23:59:59.999 → UTC ISO
    const untilDate = new Date(result.until!);
    expect(untilDate.getFullYear()).toBe(2026);
    expect(untilDate.getMonth()).toBe(6);
    expect(untilDate.getDate()).toBe(4);
    expect(untilDate.getHours()).toBe(23);
    expect(untilDate.getMinutes()).toBe(59);
    expect(untilDate.getSeconds()).toBe(59);
    expect(untilDate.getMilliseconds()).toBe(999);
  });

  it("clamps since to earliestDate when start is earlier", () => {
    const result = resolveCustomRange("2026-06-01", "2026-07-04", "2026-06-15", null);
    const sinceDate = new Date(result.since!);
    expect(sinceDate.getMonth()).toBe(5); // June = 5
    expect(sinceDate.getDate()).toBe(15);
    expect(sinceDate.getHours()).toBe(0);
  });

  it("clamps until to latestDate when end is later", () => {
    const result = resolveCustomRange("2026-07-01", "2026-12-31", null, "2026-07-04");
    const untilDate = new Date(result.until!);
    expect(untilDate.getMonth()).toBe(6); // July = 6
    expect(untilDate.getDate()).toBe(4);
    expect(untilDate.getHours()).toBe(23);
    expect(untilDate.getMinutes()).toBe(59);
    expect(untilDate.getSeconds()).toBe(59);
  });

  it("does not clamp when bounds are within range", () => {
    const result = resolveCustomRange(
      "2026-07-01",
      "2026-07-04",
      "2026-06-01",
      "2026-07-31",
    );
    const sinceDate = new Date(result.since!);
    expect(sinceDate.getDate()).toBe(1);
    expect(sinceDate.getMonth()).toBe(6);
    const untilDate = new Date(result.until!);
    expect(untilDate.getDate()).toBe(4);
    expect(untilDate.getMonth()).toBe(6);
  });
});

describe("isHourAllowed", () => {
  it("returns false when since is undefined (all history)", () => {
    expect(isHourAllowed(undefined, undefined)).toBe(false);
  });

  it("returns true for a 24h range", () => {
    const since = "2026-07-03T15:30:00.000Z";
    const until = "2026-07-04T15:30:00.000Z";
    expect(isHourAllowed(since, until)).toBe(true);
  });

  it("returns true for exactly 72h range", () => {
    const since = "2026-07-01T15:30:00.000Z";
    const until = "2026-07-04T15:30:00.000Z";
    // Exactly 72h = HOUR_BUCKET_MAX_MS, boundary is inclusive (<=)
    expect(isHourAllowed(since, until)).toBe(true);
  });

  it("returns false for a range just over 72h", () => {
    const since = "2026-07-01T15:29:59.999Z";
    const until = "2026-07-04T15:30:00.000Z";
    expect(isHourAllowed(since, until)).toBe(false);
  });

  it("returns false for a 7d range", () => {
    const since = "2026-06-27T15:30:00.000Z";
    const until = "2026-07-04T15:30:00.000Z";
    expect(isHourAllowed(since, until)).toBe(false);
  });

  it("HOUR_BUCKET_MAX_MS is 72 hours in milliseconds", () => {
    expect(HOUR_BUCKET_MAX_MS).toBe(72 * 60 * 60 * 1000);
  });
});

describe("effectiveBucket", () => {
  it("passes through non-hour buckets unchanged", () => {
    const since = "2026-06-01T00:00:00.000Z";
    const until = "2026-07-04T15:30:00.000Z";
    expect(effectiveBucket("day", since, until)).toBe("day");
    expect(effectiveBucket("week", since, until)).toBe("week");
    expect(effectiveBucket("month", since, until)).toBe("month");
  });

  it("allows hour bucket for ≤72h range", () => {
    const since = "2026-07-03T15:30:00.000Z";
    const until = "2026-07-04T15:30:00.000Z";
    expect(effectiveBucket("hour", since, until)).toBe("hour");
  });

  it("degrades hour → day when range exceeds 72h", () => {
    const since = "2026-06-27T15:30:00.000Z";
    const until = "2026-07-04T15:30:00.000Z";
    expect(effectiveBucket("hour", since, until)).toBe("day");
  });

  it("degrades hour → day when since is undefined (all history)", () => {
    expect(effectiveBucket("hour", undefined, undefined)).toBe("day");
  });
});
