import type { SeriesResponse } from "./types";

/**
 * Sort series from a SeriesResponse by total descending.
 * Returns a stable ordered list used for stacking order and shared color map.
 */
export function orderSeries(
  response: SeriesResponse,
): { name: string; total: number; points: number[] }[] {
  return [...response.series]
    .map((s) => ({
      name: s.name,
      total: s.points.reduce((acc, v) => acc + v, 0),
      points: s.points,
    }))
    .sort((a, b) => b.total - a.total);
}
