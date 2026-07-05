import type { Bucket, Metric, SeriesBy, DateRangePreset, DateRangeFilter } from "./types";
import { isHourAllowed } from "./dateRange";

export interface SegmentedControlProps<T extends string> {
  label: string;
  options: { value: T; label: string; disabled?: boolean }[];
  value: T;
  onChange: (value: T) => void;
}

export function SegmentedControl<T extends string>({
  label,
  options,
  value,
  onChange,
}: SegmentedControlProps<T>) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <span
        style={{
          fontSize: 11,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.06em",
          color: "var(--text-muted)",
        }}
      >
        {label}
      </span>
      <div
        role="group"
        aria-label={label}
        style={{
          display: "flex",
          borderRadius: "var(--radius-md)",
          border: "1px solid var(--border)",
          overflow: "hidden",
          backgroundColor: "var(--surface-2)",
        }}
      >
        {options.map((opt) => (
          <button
            key={opt.value}
            aria-pressed={opt.value === value}
            disabled={opt.disabled}
            onClick={() => !opt.disabled && onChange(opt.value)}
            style={{
              flex: 1,
              padding: "4px 10px",
              fontSize: 12,
              fontWeight: opt.value === value ? 600 : 400,
              color: opt.disabled
                ? "var(--text-subtle)"
                : opt.value === value
                  ? "var(--accent-text)"
                  : "var(--text)",
              background:
                opt.value === value && !opt.disabled ? "var(--accent)" : "transparent",
              border: "none",
              borderRadius: 0,
              cursor: opt.disabled ? "not-allowed" : "pointer",
              opacity: opt.disabled ? 0.45 : 1,
              transition: `background ${String("var(--duration-short)")}`,
              whiteSpace: "nowrap",
            }}
          >
            {opt.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export interface ChartControlsValue {
  bucket: Bucket;
  metric: Metric;
  seriesBy: SeriesBy;
  dateFilter: DateRangeFilter;
}

interface ChartControlsProps {
  value: ChartControlsValue;
  onChange: (value: ChartControlsValue) => void;
  /** Earliest available data date as "YYYY-MM-DD". Used to bound custom date inputs. */
  earliestDate?: string | null;
  /** Latest available data date as "YYYY-MM-DD". Used to bound custom date inputs. */
  latestDate?: string | null;
  /** Resolved since (UTC ISO) for the guardrail check. */
  resolvedSince?: string;
  /** Resolved until (UTC ISO) for the guardrail check. */
  resolvedUntil?: string;
}

const BUCKET_OPTIONS: { value: Bucket; label: string }[] = [
  { value: "hour", label: "Hora" },
  { value: "day", label: "Día" },
  { value: "week", label: "Semana" },
  { value: "month", label: "Mes" },
];

const METRIC_OPTIONS: { value: Metric; label: string }[] = [
  { value: "tokens", label: "Tokens" },
  { value: "cost", label: "Costo" },
];

const SERIES_BY_OPTIONS: { value: SeriesBy; label: string }[] = [
  { value: "model", label: "Modelo" },
  { value: "project", label: "Proyecto" },
  { value: "modelProject", label: "Modelo-Proyecto" },
];

const DATE_PRESET_OPTIONS: { value: DateRangePreset; label: string }[] = [
  { value: "24h", label: "24h" },
  { value: "3d", label: "3d" },
  { value: "7d", label: "7d" },
  { value: "30d", label: "30d" },
  { value: "month", label: "Este mes" },
  { value: "all", label: "Todo" },
  { value: "custom", label: "Custom" },
];

/**
 * Chart controls with Bucket, Metric, SeriesBy dimensions and a date-range
 * preset selector. Fully controlled: accepts value + onChange.
 */
export function ChartControls({
  value,
  onChange,
  earliestDate,
  latestDate,
  resolvedSince,
  resolvedUntil,
}: ChartControlsProps) {
  const hourAllowed = isHourAllowed(resolvedSince, resolvedUntil);

  const bucketOptions = BUCKET_OPTIONS.map((opt) => ({
    ...opt,
    disabled: opt.value === "hour" && !hourAllowed,
  }));

  const activePreset = value.dateFilter.preset;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-sm)",
      }}
    >
      {/* ── Row 1: date preset ──────────────────────────────────────────── */}
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        <span
          style={{
            fontSize: 11,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: "var(--text-muted)",
          }}
        >
          Rango
        </span>
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 6 }}>
          <div
            role="group"
            aria-label="Rango"
            style={{
              display: "flex",
              borderRadius: "var(--radius-md)",
              border: "1px solid var(--border)",
              overflow: "hidden",
              backgroundColor: "var(--surface-2)",
            }}
          >
            {DATE_PRESET_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                aria-pressed={opt.value === activePreset}
                onClick={() => {
                  onChange({
                    ...value,
                    dateFilter: {
                      ...value.dateFilter,
                      preset: opt.value,
                    },
                  });
                }}
                style={{
                  flex: 1,
                  padding: "4px 10px",
                  fontSize: 12,
                  fontWeight: opt.value === activePreset ? 600 : 400,
                  color:
                    opt.value === activePreset ? "var(--accent-text)" : "var(--text)",
                  background:
                    opt.value === activePreset ? "var(--accent)" : "transparent",
                  border: "none",
                  borderRadius: 0,
                  cursor: "pointer",
                  transition: `background ${String("var(--duration-short)")}`,
                  whiteSpace: "nowrap",
                }}
              >
                {opt.label}
              </button>
            ))}
          </div>

          {/* ── Custom date inputs ────────────────────────────────────────── */}
          {activePreset === "custom" && (
            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <input
                type="date"
                aria-label="Fecha inicio"
                value={value.dateFilter.customStart ?? ""}
                min={earliestDate ?? undefined}
                max={value.dateFilter.customEnd ?? latestDate ?? undefined}
                onChange={(e) =>
                  onChange({
                    ...value,
                    dateFilter: {
                      ...value.dateFilter,
                      customStart: e.target.value,
                    },
                  })
                }
                style={{
                  fontSize: 12,
                  padding: "3px 6px",
                  border: "1px solid var(--border)",
                  borderRadius: "var(--radius-md)",
                  background: "var(--surface-2)",
                  color: "var(--text)",
                  cursor: "pointer",
                }}
              />
              <span style={{ fontSize: 12, color: "var(--text-muted)" }}>–</span>
              <input
                type="date"
                aria-label="Fecha fin"
                value={value.dateFilter.customEnd ?? ""}
                min={value.dateFilter.customStart ?? earliestDate ?? undefined}
                max={latestDate ?? undefined}
                onChange={(e) =>
                  onChange({
                    ...value,
                    dateFilter: {
                      ...value.dateFilter,
                      customEnd: e.target.value,
                    },
                  })
                }
                style={{
                  fontSize: 12,
                  padding: "3px 6px",
                  border: "1px solid var(--border)",
                  borderRadius: "var(--radius-md)",
                  background: "var(--surface-2)",
                  color: "var(--text)",
                  cursor: "pointer",
                }}
              />
            </div>
          )}
        </div>
      </div>

      {/* ── Row 2: bucket / metric / series ─────────────────────────────── */}
      <div
        style={{
          display: "flex",
          gap: "var(--space-md)",
          flexWrap: "wrap",
          alignItems: "flex-start",
        }}
      >
        <SegmentedControl
          label="Período"
          options={bucketOptions}
          value={value.bucket}
          onChange={(bucket) => onChange({ ...value, bucket })}
        />
        <SegmentedControl
          label="Métrica"
          options={METRIC_OPTIONS}
          value={value.metric}
          onChange={(metric) => onChange({ ...value, metric })}
        />
        <SegmentedControl
          label="Series"
          options={SERIES_BY_OPTIONS}
          value={value.seriesBy}
          onChange={(seriesBy) => onChange({ ...value, seriesBy })}
        />
      </div>
    </div>
  );
}
