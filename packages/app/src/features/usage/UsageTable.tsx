import type { SeriesResponse } from "./types";
import { formatTokensExact, formatCost } from "./format";

interface UsageTableProps {
  response: SeriesResponse;
  orderedNames: string[];
  colorMap: Map<string, string>;
  hoveredSeries?: string | null;
  onHoverSeries?: (name: string | null) => void;
}

/**
 * Data table that shows exact token/cost values for each series × bucket.
 * Includes per-series row totals and per-bucket column totals.
 * Shares order and color map with UsageChart for visual consistency.
 */
export function UsageTable({
  response,
  orderedNames,
  colorMap,
  hoveredSeries,
  onHoverSeries,
}: UsageTableProps) {
  const { buckets, series, metric } = response;
  const isCost = metric === "cost";
  const formatter = isCost ? formatCost : formatTokensExact;

  // Build a lookup from series name → points array
  const pointsMap = new Map<string, number[]>();
  for (const s of series) {
    pointsMap.set(s.name, s.points);
  }

  // Per-series totals (row totals)
  const seriesTotal = new Map<string, number>();
  for (const name of orderedNames) {
    const pts = pointsMap.get(name) ?? [];
    seriesTotal.set(name, pts.reduce((acc, v) => acc + v, 0));
  }

  // Per-bucket totals (column totals)
  const bucketTotals = buckets.map((_, i) =>
    orderedNames.reduce((acc, name) => {
      const pts = pointsMap.get(name);
      return acc + (pts?.[i] ?? 0);
    }, 0),
  );

  const grandTotal = bucketTotals.reduce((acc, v) => acc + v, 0);

  return (
    <div className="usage-table-wrapper">
      <table className="usage-table">
        <thead>
          <tr>
            <th className="usage-table__series-cell">Serie</th>
            {buckets.map((b) => (
              <th key={b} className="usage-table__num-cell">
                {b}
              </th>
            ))}
            <th className="usage-table__num-cell usage-table__total-cell">
              Total
            </th>
          </tr>
        </thead>
        <tbody>
          {orderedNames.map((name) => {
            const pts = pointsMap.get(name) ?? [];
            const total = seriesTotal.get(name) ?? 0;
            const color = colorMap.get(name) ?? "#888";
            const isHovered = hoveredSeries === name;
            const isDimmed =
              hoveredSeries !== null &&
              hoveredSeries !== undefined &&
              !isHovered;

            return (
              <tr
                key={name}
                className={[
                  "usage-table__row",
                  isHovered ? "usage-table__row--hovered" : "",
                  isDimmed ? "usage-table__row--dimmed" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                onMouseEnter={() => onHoverSeries?.(name)}
                onMouseLeave={() => onHoverSeries?.(null)}
              >
                <td className="usage-table__series-cell">
                  <span className="usage-table__dot" style={{ background: color }} />
                  <span className="usage-table__name">{name}</span>
                </td>
                {buckets.map((b, i) => (
                  <td key={b} className="usage-table__num-cell">
                    {formatter(pts[i] ?? 0)}
                  </td>
                ))}
                <td className="usage-table__num-cell usage-table__total-cell">
                  {formatter(total)}
                </td>
              </tr>
            );
          })}
        </tbody>
        <tfoot>
          <tr className="usage-table__footer-row">
            <td className="usage-table__series-cell usage-table__footer-label">
              Total
            </td>
            {bucketTotals.map((t, i) => (
              <td
                key={buckets[i] ?? i}
                className="usage-table__num-cell usage-table__footer-label"
              >
                {formatter(t)}
              </td>
            ))}
            <td className="usage-table__num-cell usage-table__total-cell usage-table__footer-label">
              {formatter(grandTotal)}
            </td>
          </tr>
        </tfoot>
      </table>
    </div>
  );
}
