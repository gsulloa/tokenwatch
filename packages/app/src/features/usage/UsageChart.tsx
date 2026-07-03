import {
  ResponsiveContainer,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
} from "recharts";
import type { TooltipProps } from "recharts";
import type { SeriesResponse } from "./types";
import { buildColorMap } from "./colors";
import { formatTokens, formatCost, formatTokensExact, formatPercent } from "./format";
import { orderSeries } from "./seriesUtils";

// ── Custom Tooltip ───────────────────────────────────────────────────────────

interface CustomTooltipProps extends TooltipProps<number, string> {
  isCost: boolean;
  orderedNames: string[];
  colorMap: Map<string, string>;
  bucketTotals: number[];
  buckets: string[];
}

function CustomTooltip({
  active,
  payload,
  label,
  isCost,
  orderedNames,
  colorMap,
  bucketTotals,
  buckets,
}: CustomTooltipProps) {
  if (!active || !payload || payload.length === 0) return null;

  const bucketIndex = buckets.indexOf(label as string);
  const bucketTotal = bucketIndex >= 0 ? (bucketTotals[bucketIndex] ?? 0) : 0;

  // Build a map from series name → value from payload
  const valueMap = new Map<string, number>();
  for (const entry of payload) {
    if (entry.dataKey && typeof entry.value === "number") {
      valueMap.set(String(entry.dataKey), entry.value);
    }
  }

  const formatter = isCost ? formatCost : formatTokensExact;

  return (
    <div
      style={{
        background: "var(--surface)",
        border: "1px solid var(--border)",
        borderRadius: "var(--radius-md)",
        padding: "var(--space-xs) var(--space-sm)",
        fontSize: 12,
        minWidth: 180,
        boxShadow: "var(--shadow-md)",
      }}
    >
      <div
        style={{
          fontWeight: 600,
          marginBottom: "var(--space-2xs)",
          color: "var(--text)",
          borderBottom: "1px solid var(--border)",
          paddingBottom: "var(--space-2xs)",
        }}
      >
        {label}
        <span
          style={{
            float: "right",
            fontWeight: 500,
            color: "var(--text-muted)",
          }}
        >
          Total: {formatter(bucketTotal)}
        </span>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
        {orderedNames.map((name) => {
          const value = valueMap.get(name) ?? 0;
          const color = colorMap.get(name) ?? "#888";
          return (
            <div
              key={name}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              <span
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: "50%",
                  background: color,
                  flexShrink: 0,
                }}
              />
              <span
                style={{
                  flex: 1,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                  color: "var(--text-muted)",
                }}
              >
                {name}
              </span>
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  color: "var(--text)",
                  marginLeft: "var(--space-xs)",
                  flexShrink: 0,
                }}
              >
                {formatter(value)}
              </span>
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  color: "var(--text-subtle)",
                  fontSize: 11,
                  flexShrink: 0,
                  minWidth: 44,
                  textAlign: "right",
                }}
              >
                {formatPercent(value, bucketTotal)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ── Custom Legend ────────────────────────────────────────────────────────────

interface CustomLegendProps {
  orderedNames: string[];
  colorMap: Map<string, string>;
  hoveredSeries?: string | null;
  onHoverSeries?: (name: string | null) => void;
}

function CustomLegend({
  orderedNames,
  colorMap,
  hoveredSeries,
  onHoverSeries,
}: CustomLegendProps) {
  return (
    <div
      style={{
        display: "flex",
        flexWrap: "wrap",
        gap: "var(--space-xs)",
        paddingTop: "var(--space-xs)",
      }}
    >
      {orderedNames.map((name) => {
        const color = colorMap.get(name) ?? "#888";
        const isHovered = hoveredSeries === name;
        const isDimmed = hoveredSeries !== null && !isHovered;
        return (
          <div
            key={name}
            onMouseEnter={() => onHoverSeries?.(name)}
            onMouseLeave={() => onHoverSeries?.(null)}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 4,
              opacity: isDimmed ? 0.35 : 1,
              transition: `opacity var(--duration-short)`,
              cursor: "default",
            }}
          >
            <span
              style={{
                width: 8,
                height: 8,
                borderRadius: "50%",
                background: color,
                flexShrink: 0,
              }}
            />
            <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
              {name}
            </span>
          </div>
        );
      })}
    </div>
  );
}

// ── UsageChart ───────────────────────────────────────────────────────────────

interface UsageChartProps {
  response: SeriesResponse;
  colorMap?: Map<string, string>;
  orderedNames?: string[];
  hoveredSeries?: string | null;
  onHoverSeries?: (name: string | null) => void;
}

/**
 * Recharts stacked area chart that renders one area per series from a SeriesResponse.
 * All series are shown individually (no "Otros" grouping).
 * Series are stacked so the total height at each bucket equals the sum of all series.
 */
export function UsageChart({
  response,
  colorMap: colorMapProp,
  orderedNames: orderedNamesProp,
  hoveredSeries,
  onHoverSeries,
}: UsageChartProps) {
  const isCost = response.metric === "cost";
  const isEmpty =
    response.buckets.length === 0 ||
    response.series.every((s) => s.points.every((p) => p === 0));

  if (isEmpty || response.series.length === 0) {
    return (
      <div
        role="status"
        aria-label="Sin datos"
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          height: 360,
          gap: 8,
          color: "var(--text-muted)",
        }}
      >
        <span style={{ fontSize: 32 }}>📊</span>
        <p style={{ margin: 0, fontWeight: 600 }}>Sin datos de uso</p>
        <p style={{ margin: 0, fontSize: 12 }}>
          Aún no hay eventos de uso registrados para mostrar.
        </p>
      </div>
    );
  }

  // Compute ordered names + color map either from props or internally
  const sorted = orderSeries(response);
  const orderedNames = orderedNamesProp ?? sorted.map((s) => s.name);
  const colorMap = colorMapProp ?? buildColorMap(orderedNames);

  // Build recharts rows: one object per bucket
  const rows: Record<string, string | number>[] = response.buckets.map(
    (bucket, i) => {
      const row: Record<string, string | number> = { bucket };
      for (const s of sorted) {
        row[s.name] = s.points[i] ?? 0;
      }
      return row;
    },
  );

  // Per-bucket totals (for tooltip percentage)
  const bucketTotals = response.buckets.map((_, i) =>
    sorted.reduce((acc, s) => acc + (s.points[i] ?? 0), 0),
  );

  const axisFormatter = isCost ? formatCost : formatTokens;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      {isCost && (
        <p
          role="note"
          style={{
            margin: 0,
            fontSize: 11,
            color: "var(--warning)",
            fontWeight: 500,
          }}
        >
          * Costo estimado basado en la tabla de precios de Anthropic. Los
          valores reales pueden variar.
        </p>
      )}

      <ResponsiveContainer width="100%" height={360}>
        <AreaChart
          data={rows}
          margin={{ top: 4, right: 16, bottom: 0, left: 0 }}
        >
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
          <XAxis
            dataKey="bucket"
            tick={{ fontSize: 11, fill: "var(--text-muted)" }}
            tickLine={false}
          />
          <YAxis
            tickFormatter={axisFormatter}
            tick={{ fontSize: 11, fill: "var(--text-muted)" }}
            tickLine={false}
            axisLine={false}
            width={60}
          />
          <Tooltip
            content={
              <CustomTooltip
                isCost={isCost}
                orderedNames={orderedNames}
                colorMap={colorMap}
                bucketTotals={bucketTotals}
                buckets={response.buckets}
              />
            }
          />
          {/* Render areas in reverse order so largest is at the bottom (base) */}
          {[...orderedNames].reverse().map((name) => {
            const color = colorMap.get(name) ?? "#888";
            const isHovered = hoveredSeries === name;
            const isDimmed =
              hoveredSeries !== null && hoveredSeries !== undefined && !isHovered;
            return (
              <Area
                key={name}
                type="monotone"
                dataKey={name}
                stackId="usage"
                stroke={color}
                strokeWidth={1.5}
                fill={color}
                fillOpacity={isDimmed ? 0.1 : 0.75}
                strokeOpacity={isDimmed ? 0.2 : 1}
                dot={false}
                activeDot={{ r: 3 }}
                onMouseEnter={() => onHoverSeries?.(name)}
                onMouseLeave={() => onHoverSeries?.(null)}
              />
            );
          })}
        </AreaChart>
      </ResponsiveContainer>

      <CustomLegend
        orderedNames={orderedNames}
        colorMap={colorMap}
        hoveredSeries={hoveredSeries}
        onHoverSeries={onHoverSeries}
      />
    </div>
  );
}
