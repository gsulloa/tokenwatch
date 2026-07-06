/** Temporal bucket granularity for time-series queries. */
export type Bucket = "hour" | "day" | "week" | "month";

/** Metric to aggregate in the chart. */
export type Metric = "tokens" | "cost";

/** Dimension to split series by. */
export type SeriesBy = "model" | "project" | "modelProject" | "group";

/** Date range preset for the chart filter control. */
export type DateRangePreset =
  | "24h"
  | "3d"
  | "7d"
  | "30d"
  | "month"
  | "all"
  | "custom";

/**
 * Filter state for the date-range controls.
 * For "custom", customStart/customEnd are local date strings ("YYYY-MM-DD").
 */
export interface DateRangeFilter {
  preset: DateRangePreset;
  customStart?: string;
  customEnd?: string;
}

/** Parameters for a time-series query to the Tauri backend. */
export interface SeriesQuery {
  bucket: Bucket;
  metric: Metric;
  seriesBy: SeriesBy;
  /**
   * Start of the query range as a full ISO 8601 UTC datetime (e.g. "2026-07-04T00:00:00Z").
   * If omitted, the query covers the full available history.
   */
  since?: string;
  /**
   * End of the query range as a full ISO 8601 UTC datetime (e.g. "2026-07-04T23:59:59Z").
   * If omitted, the query covers up to the latest available event.
   */
  until?: string;
}

/** A single named series with one value per bucket (missing buckets are 0). */
export interface Series {
  name: string;
  points: number[];
}

/** Response from query_series command. */
export interface SeriesResponse {
  buckets: string[];
  series: Series[];
  metric: Metric;
  bucket: Bucket;
}

/** Metadata about the usage store. Mirrors the Rust `UsageMeta` (camelCase). */
export interface UsageMeta {
  lastRefreshAt: string | null;
  eventCount: number;
  earliestDate: string | null;
  latestDate: string | null;
}

/** Summary returned by refresh_usage command. */
export interface RefreshSummary {
  filesScanned: number;
  filesIngested: number;
  eventsAdded: number;
}
