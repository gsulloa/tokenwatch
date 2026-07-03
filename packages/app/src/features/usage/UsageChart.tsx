import {
  ResponsiveContainer,
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
} from "recharts";
import type { SeriesResponse, Series } from "./types";
import { colorForSeries } from "./colors";
import { formatTokens, formatCost } from "./format";

/** Maximum number of series to display individually before grouping into "Otros". */
const TOP_N = 8;

/** The name shown for the aggregated remainder series. */
const OTROS_LABEL = "Otros";

interface ProcessedData {
  /** Row data for recharts: [{ bucket: string, [seriesName]: number }] */
  rows: Record<string, string | number>[];
  /** The series names to actually render as lines (top-N + maybe "Otros") */
  visibleSeriesNames: string[];
  /** How many series were grouped into "Otros" (0 if no grouping needed) */
  otrosCount: number;
}

/**
 * Transform a SeriesResponse into recharts row data, applying top-N grouping.
 */
function processSeriesResponse(response: SeriesResponse): ProcessedData {
  const { buckets, series } = response;

  // Sort series by total descending
  const sorted = [...series].sort((a, b) => {
    const sumA = a.points.reduce((acc, v) => acc + v, 0);
    const sumB = b.points.reduce((acc, v) => acc + v, 0);
    return sumB - sumA;
  });

  const topSeries = sorted.slice(0, TOP_N);
  const otrosSeries = sorted.slice(TOP_N);
  const otrosCount = otrosSeries.length;

  // Build visible series (top-N + optional "Otros")
  const visibleSeries: Series[] = [...topSeries];
  if (otrosCount > 0) {
    // Aggregate the remainder into a single "Otros" series
    const otrosPoints = buckets.map((_, i) =>
      otrosSeries.reduce((acc, s) => acc + (s.points[i] ?? 0), 0),
    );
    visibleSeries.push({ name: OTROS_LABEL, points: otrosPoints });
  }

  // Build recharts rows: one object per bucket
  const rows: Record<string, string | number>[] = buckets.map((bucket, i) => {
    const row: Record<string, string | number> = { bucket };
    for (const s of visibleSeries) {
      row[s.name] = s.points[i] ?? 0;
    }
    return row;
  });

  return {
    rows,
    visibleSeriesNames: visibleSeries.map((s) => s.name),
    otrosCount,
  };
}

interface UsageChartProps {
  response: SeriesResponse;
}

/**
 * Recharts line chart that renders one line per series from a SeriesResponse.
 * Applies top-N grouping and shows how many series were grouped into "Otros".
 */
export function UsageChart({ response }: UsageChartProps) {
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
          height: 240,
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

  const { rows, visibleSeriesNames, otrosCount } =
    processSeriesResponse(response);

  const formatter = isCost ? formatCost : formatTokens;

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
      {otrosCount > 0 && (
        <p
          style={{
            margin: 0,
            fontSize: 11,
            color: "var(--text-muted)",
          }}
        >
          Mostrando las top {TOP_N} series. {otrosCount} serie
          {otrosCount !== 1 ? "s" : ""} agrupada{otrosCount !== 1 ? "s" : ""}{" "}
          en &ldquo;{OTROS_LABEL}&rdquo;.
        </p>
      )}
      <ResponsiveContainer width="100%" height={240}>
        <LineChart
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
            tickFormatter={formatter}
            tick={{ fontSize: 11, fill: "var(--text-muted)" }}
            tickLine={false}
            axisLine={false}
            width={60}
          />
          <Tooltip
            formatter={(value: number) => [formatter(value)]}
            contentStyle={{
              background: "var(--surface)",
              border: "1px solid var(--border)",
              borderRadius: "var(--radius-md)",
              fontSize: 12,
            }}
          />
          <Legend
            wrapperStyle={{ fontSize: 11, paddingTop: 8 }}
            iconType="plainline"
          />
          {visibleSeriesNames.map((name) => (
            <Line
              key={name}
              type="monotone"
              dataKey={name}
              stroke={colorForSeries(name)}
              strokeWidth={name === OTROS_LABEL ? 1.5 : 2}
              strokeDasharray={name === OTROS_LABEL ? "4 2" : undefined}
              dot={false}
              activeDot={{ r: 4 }}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
