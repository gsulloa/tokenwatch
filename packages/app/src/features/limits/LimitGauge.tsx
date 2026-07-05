const TWELVE_HOURS_MS = 12 * 60 * 60 * 1000;

/**
 * Compute "resets in Xh Ym" from a future ISO 8601 timestamp.
 * Returns a short human-readable string.
 */
function formatTimeUntilReset(resetsAt: string): string {
  if (!resetsAt) return "";
  try {
    const now = Date.now();
    const target = new Date(resetsAt).getTime();
    const diffMs = target - now;
    if (diffMs <= 0) return "resetea ahora";

    const totalMinutes = Math.floor(diffMs / 60_000);
    const hours = Math.floor(totalMinutes / 60);
    const minutes = totalMinutes % 60;

    if (hours > 0 && minutes > 0) return `resetea en ${hours}h ${minutes}m`;
    if (hours > 0) return `resetea en ${hours}h`;
    return `resetea en ${minutes}m`;
  } catch {
    return "";
  }
}

/**
 * Format the reset moment as a local clock time with its timezone name,
 * e.g. "20:00 GMT-4".
 */
function formatResetClock(resetsAt: string): string {
  if (!resetsAt) return "";
  try {
    return new Date(resetsAt).toLocaleTimeString("es", {
      hour: "2-digit",
      minute: "2-digit",
      timeZoneName: "short",
    });
  } catch {
    return "";
  }
}

/**
 * Format the reset moment as a closing date in Spanish,
 * e.g. "cierra el sáb 12 jul".
 */
function formatResetDate(resetsAt: string): string {
  if (!resetsAt) return "";
  try {
    return (
      "cierra el " +
      new Date(resetsAt).toLocaleDateString("es", {
        weekday: "short",
        day: "numeric",
        month: "short",
      })
    );
  } catch {
    return "";
  }
}

/**
 * Pick a bar fill color based on utilization severity.
 * >=80 → danger, >=50 → warning, else → accent.
 */
function barColor(utilization: number): string {
  if (utilization >= 80) return "var(--danger)";
  if (utilization >= 50) return "var(--warning)";
  return "var(--accent)";
}

interface LimitGaugeProps {
  label: string;
  /** Utilization percentage 0–100. */
  utilization: number;
  /** ISO 8601 timestamp when this window resets. */
  resetsAt: string;
  /** Render a smaller, more compact variant for per-model rows. */
  compact?: boolean;
}

/**
 * A labeled horizontal progress bar with % and "resetea en Xh Ym".
 */
export function LimitGauge({
  label,
  utilization,
  resetsAt,
  compact = false,
}: LimitGaugeProps) {
  const clampedPct = Math.min(100, Math.max(0, utilization));
  const diffMs = resetsAt ? new Date(resetsAt).getTime() - Date.now() : NaN;
  const resetLabel =
    resetsAt && !isNaN(diffMs) && diffMs >= TWELVE_HOURS_MS
      ? formatResetDate(resetsAt)
      : [formatTimeUntilReset(resetsAt), formatResetClock(resetsAt)]
          .filter(Boolean)
          .join(" · ");
  const color = barColor(clampedPct);

  const trackHeight = compact ? 4 : 6;
  const fontSize = compact ? 11 : 12;
  const labelFontSize = compact ? 11 : 12;
  const verticalGap = compact ? 3 : 5;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: verticalGap,
      }}
    >
      {/* Header row: label left, pct right */}
      <div
        style={{
          display: "flex",
          alignItems: "baseline",
          justifyContent: "space-between",
        }}
      >
        <span
          style={{
            fontSize: labelFontSize,
            fontWeight: compact ? 400 : 600,
            color: "var(--text)",
          }}
        >
          {label}
        </span>
        <span
          style={{
            fontSize,
            fontWeight: 600,
            color,
            fontVariantNumeric: "tabular-nums",
          }}
        >
          {Math.round(clampedPct)}%
        </span>
      </div>

      {/* Track + fill */}
      <div
        role="progressbar"
        aria-valuenow={Math.round(clampedPct)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label}
        style={{
          position: "relative",
          height: trackHeight,
          borderRadius: "var(--radius-full)",
          background: "var(--surface-3)",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            position: "absolute",
            inset: "0 auto 0 0",
            width: `${clampedPct}%`,
            background: color,
            borderRadius: "var(--radius-full)",
            transition: "width var(--duration-medium) ease",
          }}
        />
      </div>

      {/* Reset label: "resetea en Xh Ym · 20:00 GMT-4" when < 12 h remain; "cierra el sáb 12 jul" otherwise */}
      {resetLabel && (
        <span
          style={{
            fontSize: 10,
            color: "var(--text-muted)",
            lineHeight: 1,
          }}
        >
          {resetLabel}
        </span>
      )}
    </div>
  );
}
