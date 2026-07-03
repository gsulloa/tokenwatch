import { useState, useMemo } from "react";
import { APP_DISPLAY_NAME } from "@/platform/app-identity";
import { ChartControls } from "@/features/usage/ChartControls";
import { UsageChart } from "@/features/usage/UsageChart";
import { UsageTable } from "@/features/usage/UsageTable";
import { useUsageSeries } from "@/features/usage/useUsageSeries";
import { buildColorMap } from "@/features/usage/colors";
import { orderSeries } from "@/features/usage/seriesUtils";
import { formatTokens, formatTokensExact, formatCost } from "@/features/usage/format";
import type { ChartControlsValue } from "@/features/usage/ChartControls";

const DEFAULT_CONTROLS: ChartControlsValue = {
  bucket: "day",
  metric: "tokens",
  seriesBy: "model",
};

function formatRefreshAt(iso: string | null | undefined): string {
  if (!iso) return "Nunca";
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

const SERIES_BY_LABELS: Record<ChartControlsValue["seriesBy"], string> = {
  model: "Modelo",
  project: "Proyecto",
  modelProject: "Modelo-Proyecto",
};

const BUCKET_LABELS: Record<ChartControlsValue["bucket"], string> = {
  day: "Días",
  week: "Semanas",
  month: "Meses",
};

// ── KPI Card ─────────────────────────────────────────────────────────────────

interface KpiCardProps {
  label: string;
  value: string;
  sub?: string;
  title?: string;
  monoValue?: boolean;
}

function KpiCard({ label, value, sub, title, monoValue }: KpiCardProps) {
  return (
    <div className="kpi-card">
      <span className="kpi-card__label">{label}</span>
      <span
        className={monoValue ? "kpi-card__value kpi-card__value--mono" : "kpi-card__value"}
        title={title}
      >
        {value}
      </span>
      {sub && <span className="kpi-card__sub">{sub}</span>}
    </div>
  );
}

// ── App ───────────────────────────────────────────────────────────────────────

export function App() {
  const [controls, setControls] =
    useState<ChartControlsValue>(DEFAULT_CONTROLS);

  const [hoveredSeries, setHoveredSeries] = useState<string | null>(null);

  const { data, meta, loading, error, refresh } = useUsageSeries({
    bucket: controls.bucket,
    metric: controls.metric,
    seriesBy: controls.seriesBy,
  });

  // Compute the ordered names and color map ONCE at App level so chart + table
  // share identical colors and series order.
  const { orderedNames, colorMap } = useMemo(() => {
    if (!data) return { orderedNames: [], colorMap: new Map<string, string>() };
    const sorted = orderSeries(data);
    const names = sorted.map((s) => s.name);
    return { orderedNames: names, colorMap: buildColorMap(names) };
  }, [data]);

  // Only show the table when there is data and at least one non-zero series
  const hasNonZeroData =
    data !== null &&
    data !== undefined &&
    data.series.some((s) => s.points.some((p) => p !== 0));

  // ── KPI computations ─────────────────────────────────────────────────────

  const kpiTotal = useMemo(() => {
    if (!data) return 0;
    return data.series.reduce(
      (acc, s) => acc + s.points.reduce((a, v) => a + v, 0),
      0,
    );
  }, [data]);

  const kpiTotalLabel =
    controls.metric === "cost" ? "Costo total" : "Total tokens";

  const kpiTotalValue =
    data === null
      ? "—"
      : controls.metric === "cost"
        ? formatCost(kpiTotal)
        : formatTokens(kpiTotal);

  const kpiTotalTitle =
    data !== null && controls.metric === "tokens"
      ? formatTokensExact(kpiTotal)
      : undefined;

  const kpiSeriesCount = data?.series.length ?? 0;
  const kpiSeriesSub = SERIES_BY_LABELS[controls.seriesBy];

  const kpiBucketCount = data?.buckets.length ?? 0;
  const kpiBucketSub = BUCKET_LABELS[controls.bucket];

  const kpiEventCount = meta?.eventCount ?? 0;

  const kpiDateMin = meta?.earliestDate ?? null;
  const kpiDateMax = meta?.latestDate ?? null;
  const kpiDateRange =
    kpiDateMin && kpiDateMax ? `${kpiDateMin} – ${kpiDateMax}` : "—";

  return (
    <div className="dashboard">
      {/* ── Top App Bar ─────────────────────────────────────────────────── */}
      <header className="dashboard__topbar">
        <div className="dashboard__topbar-inner">
          <div>
            <h1 className="dashboard__app-title">{APP_DISPLAY_NAME}</h1>
            <p className="dashboard__app-subtitle">
              Uso de tokens de Claude / Codex por proyecto
            </p>
          </div>
          <div className="dashboard__topbar-actions">
            <span className="dashboard__refresh-time">
              Actualizado: {formatRefreshAt(meta?.lastRefreshAt)}
            </span>
            <button
              className="dashboard__refresh-btn"
              onClick={refresh}
              disabled={loading}
              aria-label="Actualizar datos"
            >
              {loading ? "Cargando…" : "Actualizar"}
            </button>
          </div>
        </div>
      </header>

      {/* ── Main content column ──────────────────────────────────────────── */}
      <div className="dashboard__content">
        {/* Error alert */}
        {error && (
          <p role="alert" className="dashboard__error">
            Error al cargar datos: {error}
          </p>
        )}

        {/* ── KPI Summary Row ─────────────────────────────────────────────── */}
        <div className="kpi-grid">
          <KpiCard
            label={kpiTotalLabel}
            value={kpiTotalValue}
            title={kpiTotalTitle}
          />
          <KpiCard
            label="Series"
            value={data === null ? "—" : String(kpiSeriesCount)}
            sub={kpiSeriesSub}
          />
          <KpiCard
            label="Períodos"
            value={data === null ? "—" : String(kpiBucketCount)}
            sub={kpiBucketSub}
          />
          <KpiCard
            label="Eventos"
            value={String(kpiEventCount)}
          />
          <KpiCard
            label="Rango de fechas"
            value={kpiDateRange}
            monoValue={kpiDateRange !== "—"}
          />
        </div>

        {/* ── Controls Toolbar ────────────────────────────────────────────── */}
        <div className="toolbar">
          <ChartControls value={controls} onChange={setControls} />
        </div>

        {/* ── Chart Panel ─────────────────────────────────────────────────── */}
        <div className="panel">
          <div className="panel__header">
            <h2 className="panel__title">Uso a lo largo del tiempo</h2>
          </div>
          <div className="panel__body">
            {data ? (
              <UsageChart
                response={data}
                colorMap={colorMap}
                orderedNames={orderedNames}
                hoveredSeries={hoveredSeries}
                onHoverSeries={setHoveredSeries}
              />
            ) : !loading ? (
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
                  Conecta con el backend de Tauri para ver datos de uso.
                </p>
              </div>
            ) : null}
          </div>
        </div>

        {/* ── Table Panel ─────────────────────────────────────────────────── */}
        {hasNonZeroData && (
          <div className="panel">
            <div className="panel__header">
              <h2 className="panel__title">Detalle por serie</h2>
            </div>
            <div className="panel__body panel__body--flush">
              <UsageTable
                response={data!}
                orderedNames={orderedNames}
                colorMap={colorMap}
                hoveredSeries={hoveredSeries}
                onHoverSeries={setHoveredSeries}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
