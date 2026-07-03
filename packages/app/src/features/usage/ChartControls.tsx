import type { Bucket, Metric, SeriesBy } from "./types";

interface SegmentedControlProps<T extends string> {
  label: string;
  options: { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
}

function SegmentedControl<T extends string>({
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
            onClick={() => onChange(opt.value)}
            style={{
              flex: 1,
              padding: "4px 10px",
              fontSize: 12,
              fontWeight: opt.value === value ? 600 : 400,
              color: opt.value === value ? "var(--accent-text)" : "var(--text)",
              background:
                opt.value === value ? "var(--accent)" : "transparent",
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
    </div>
  );
}

export interface ChartControlsValue {
  bucket: Bucket;
  metric: Metric;
  seriesBy: SeriesBy;
}

interface ChartControlsProps {
  value: ChartControlsValue;
  onChange: (value: ChartControlsValue) => void;
}

const BUCKET_OPTIONS: { value: Bucket; label: string }[] = [
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

/**
 * Three segmented controls for Bucket, Metric, and SeriesBy dimensions.
 * Fully controlled: accepts value + onChange.
 */
export function ChartControls({ value, onChange }: ChartControlsProps) {
  return (
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
        options={BUCKET_OPTIONS}
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
  );
}
