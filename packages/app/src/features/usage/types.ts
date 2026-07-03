/** Temporal bucket granularity for time-series queries. */
export type Bucket = "day" | "week" | "month";

/** Metric to aggregate in the chart. */
export type Metric = "tokens" | "cost";

/** Dimension to split series by. */
export type SeriesBy = "model" | "project" | "modelProject";

/** Parameters for a time-series query to the Tauri backend. */
export interface SeriesQuery {
  bucket: Bucket;
  metric: Metric;
  seriesBy: SeriesBy;
  since?: string;
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
