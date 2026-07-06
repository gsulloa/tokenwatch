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

function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
}

/**
 * Pick a bar fill color based on utilization severity.
 * >=100 → critical, >=85 → danger, >=70 → watch, else → safe.
 */
function barColor(utilization: number): string {
  if (utilization >= 100) return "var(--gauge-critical)";
  if (utilization >= 85) return "var(--gauge-danger)";
  if (utilization >= 70) return "var(--gauge-watch)";
  return "var(--gauge-safe)";
}

interface LimitGaugeProps {
  label: string;
  /** Utilization percentage 0–100. */
  utilization: number;
  /** ISO 8601 timestamp when this window resets. */
  resetsAt: string;
  /** Render a smaller, more compact variant for per-model rows. */
  compact?: boolean;
  /**
   * Total window duration in ms. When provided along with a valid resetsAt,
   * the component computes a pace marker and pace note.
   */
  windowMs?: number;
}

/**
 * A labeled horizontal progress bar with % and "resetea en Xh Ym".
 * Non-compact gauges show threshold ticks at 70%, 85%, 100%
 * and an optional pace marker when windowMs is supplied.
 */
export function LimitGauge({
  label,
  utilization,
  resetsAt,
  compact = false,
  windowMs,
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

  // Pace marker computation
  let pace: number | null = null;
  let paceRatio: number | null = null;

  if (windowMs && resetsAt) {
    const msUntilReset = new Date(resetsAt).getTime() - Date.now();
    if (!isNaN(msUntilReset)) {
      pace = clamp01((windowMs - msUntilReset) / windowMs);
      paceRatio = pace > 0 ? (clampedPct / 100) / pace : null;
    }
  }

  const trackHeight = compact ? 5 : 8;
  const markerHeight = trackHeight + 4; // slightly taller than the rail
  const fontSize = compact ? 11 : 12;
  const labelFontSize = compact ? 11 : 12;
  const verticalGap = compact ? 3 : 5;

  // Pace note text and color
  let paceNote: string | null = null;
  let paceNoteColor: string = "var(--text-muted)";
  if (paceRatio !== null) {
    if (paceRatio >= 1.15) {
      paceNote = `${paceRatio.toFixed(1)}× sobre ritmo`;
      paceNoteColor = color;
    } else if (paceRatio <= 0.85) {
      paceNote = "holgado";
      paceNoteColor = "var(--gauge-safe)";
    } else {
      paceNote = "en ritmo";
      paceNoteColor = "var(--text-muted)";
    }
  }

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
            fontSize: compact ? fontSize : 15,
            fontWeight: 500,
            color,
            fontVariantNumeric: "tabular-nums",
            fontFamily: "var(--font-data)",
          }}
        >
          {Math.round(clampedPct)}%
        </span>
      </div>

      {/* Track + fill + ticks + pace marker */}
      <div
        role="progressbar"
        aria-valuenow={Math.round(clampedPct)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label}
        style={{
          position: "relative",
          height: trackHeight,
          borderRadius: "var(--radius-sm)",
          background: "var(--track)",
        }}
      >
        {/* Fill */}
        <div
          style={{
            position: "absolute",
            inset: "0 auto 0 0",
            width: `${clampedPct}%`,
            background: color,
            borderRadius: "var(--radius-sm)",
          }}
        />

        {/* Threshold ticks at 70%, 85%, 100% — non-compact only */}
        {!compact && (
          <>
            <div
              aria-hidden
              style={{
                position: "absolute",
                top: 0,
                bottom: 0,
                left: "70%",
                width: 1,
                background: "var(--border-strong)",
              }}
            />
            <div
              aria-hidden
              style={{
                position: "absolute",
                top: 0,
                bottom: 0,
                left: "85%",
                width: 1,
                background: "var(--border-strong)",
              }}
            />
            <div
              aria-hidden
              style={{
                position: "absolute",
                top: 0,
                bottom: 0,
                left: "100%",
                width: 1,
                background: "var(--border-strong)",
              }}
            />
          </>
        )}

        {/* Pace marker — neutral high-contrast, never brand/purple */}
        {pace !== null && (
          <div
            aria-hidden
            style={{
              position: "absolute",
              top: "50%",
              transform: "translate(-50%, -50%)",
              left: `${pace * 100}%`,
              width: 2,
              height: markerHeight,
              background: "rgba(244, 242, 248, 0.85)",
              borderRadius: 1,
            }}
          />
        )}
      </div>

      {/* Sub-row: pace note (left) + reset label (right) */}
      {(paceNote !== null || resetLabel) && (
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            gap: 4,
          }}
        >
          {paceNote !== null ? (
            <span
              style={{
                fontSize: 10,
                color: paceNoteColor,
                lineHeight: 1,
              }}
            >
              {paceNote}
            </span>
          ) : (
            <span />
          )}
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
      )}

    </div>
  );
}
