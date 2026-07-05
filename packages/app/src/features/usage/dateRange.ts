import type { Bucket, DateRangePreset } from "./types";

/** Span in milliseconds beyond which the "hour" bucket is disallowed. */
export const HOUR_BUCKET_MAX_MS = 72 * 60 * 60 * 1000; // 72 hours

export interface ResolvedRange {
  since?: string;
  until?: string;
  defaultBucket: Bucket;
}

/**
 * Compute the date range for a given preset relative to `now`.
 * Pass `now` as an argument (do NOT read the clock inside this function) so
 * the function is fully testable.
 *
 * Returns ISO 8601 UTC strings (e.g. "2026-07-04T00:00:00.000Z") for
 * `since`/`until`, or undefined for "all" (no filter).
 */
export function resolvePreset(preset: DateRangePreset, now: Date): ResolvedRange {
  const until = now.toISOString();

  switch (preset) {
    case "24h": {
      const since = new Date(now.getTime() - 24 * 60 * 60 * 1000).toISOString();
      return { since, until, defaultBucket: "hour" };
    }
    case "3d": {
      const since = new Date(now.getTime() - 3 * 24 * 60 * 60 * 1000).toISOString();
      return { since, until, defaultBucket: "hour" };
    }
    case "7d": {
      const since = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000).toISOString();
      return { since, until, defaultBucket: "day" };
    }
    case "30d": {
      const since = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000).toISOString();
      return { since, until, defaultBucket: "day" };
    }
    case "month": {
      // Start of the current month in local time → convert to UTC ISO
      const localStartOfMonth = new Date(
        now.getFullYear(),
        now.getMonth(),
        1,
        0,
        0,
        0,
        0,
      );
      return { since: localStartOfMonth.toISOString(), until, defaultBucket: "day" };
    }
    case "all": {
      return { since: undefined, until: undefined, defaultBucket: "week" };
    }
    case "custom": {
      // Custom ranges are resolved separately via resolveCustomRange.
      // Return a safe default if called directly.
      return { since: undefined, until: undefined, defaultBucket: "day" };
    }
  }
}

/**
 * Resolve a custom date range from two local date strings ("YYYY-MM-DD"),
 * clamped to the available data bounds (earliestDate/latestDate from UsageMeta).
 *
 * `start` is treated as local 00:00:00 of that day.
 * `end`   is treated as local 23:59:59.999 of that day.
 *
 * Pass null/undefined for bounds to skip clamping.
 */
export function resolveCustomRange(
  start: string,
  end: string,
  earliestDate: string | null | undefined,
  latestDate: string | null | undefined,
): ResolvedRange {
  // Parse start/end as local dates
  const [sy, sm, sd] = start.split("-").map(Number) as [number, number, number];
  const [ey, em, ed] = end.split("-").map(Number) as [number, number, number];

  let sinceMs = new Date(sy, sm - 1, sd, 0, 0, 0, 0).getTime();
  let untilMs = new Date(ey, em - 1, ed, 23, 59, 59, 999).getTime();

  // Clamp to data bounds when available
  if (earliestDate) {
    const [eby, ebm, ebd] = earliestDate.split("-").map(Number) as [number, number, number];
    const earliestMs = new Date(eby, ebm - 1, ebd, 0, 0, 0, 0).getTime();
    if (sinceMs < earliestMs) sinceMs = earliestMs;
  }
  if (latestDate) {
    const [lby, lbm, lbd] = latestDate.split("-").map(Number) as [number, number, number];
    const latestMs = new Date(lby, lbm - 1, lbd, 23, 59, 59, 999).getTime();
    if (untilMs > latestMs) untilMs = latestMs;
  }

  return {
    since: new Date(sinceMs).toISOString(),
    until: new Date(untilMs).toISOString(),
    defaultBucket: "day",
  };
}

/**
 * Returns true when the "hour" bucket is allowed for the given range.
 * "hour" is disallowed when the span exceeds HOUR_BUCKET_MAX_MS (~72 hours).
 * If `since` is undefined (all history), hour is not allowed.
 */
export function isHourAllowed(since: string | undefined, until: string | undefined): boolean {
  if (!since) return false;
  const sinceMs = new Date(since).getTime();
  const untilMs = until ? new Date(until).getTime() : Date.now();
  return untilMs - sinceMs <= HOUR_BUCKET_MAX_MS;
}

/**
 * Given a desired bucket and the active range, returns the effective bucket.
 * Degrades "hour" → "day" when the guardrail disallows hourly granularity.
 */
export function effectiveBucket(
  desired: Bucket,
  since: string | undefined,
  until: string | undefined,
): Bucket {
  if (desired === "hour" && !isHourAllowed(since, until)) {
    return "day";
  }
  return desired;
}
