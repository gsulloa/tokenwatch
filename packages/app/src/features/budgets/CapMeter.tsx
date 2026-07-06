import { formatCost } from "@/features/usage/format";
import type { BudgetBasis } from "./types";

/**
 * Pick a bar fill color based on proximity to the group's own cap.
 * >= 100% (over cap) → critical; >= 85% → danger; >= 70% → watch; else → safe.
 */
function capBarColor(fraction: number): string {
  if (fraction >= 1) return "var(--gauge-critical)";
  if (fraction >= 0.85) return "var(--gauge-danger)";
  if (fraction >= 0.7) return "var(--gauge-watch)";
  return "var(--gauge-safe)";
}

/**
 * Format the readout text with the honest unit.
 * share (session): "18% / 30% sesión (est.)"
 * usd:             "$2.10 / $2.00"
 */
function formatReadout(
  measuredValue: number,
  budgetValue: number,
  budgetBasis: BudgetBasis,
): string {
  if (budgetBasis === "share") {
    return `${Math.round(measuredValue)}% / ${Math.round(budgetValue)}% sesión (est.)`;
  }
  return `${formatCost(measuredValue)} / ${formatCost(budgetValue)}`;
}

interface CapMeterProps {
  measuredValue: number;
  budgetValue: number;
  budgetBasis: BudgetBasis;
  /** Accessible label (group name) */
  label: string;
}

/**
 * A thin progress bar whose fill fraction = measuredValue / budgetValue (clamped 0..1).
 * Readout carries the honest unit: share → "18% / 30% costo local", usd → "$2.10 / $2.00".
 * Color is by proximity to the group's OWN cap (reuses LimitGauge tone thresholds,
 * but visually distinct: thinner 3px track, NO reset clock).
 * Over-cap: bar clamps 100%, value is red.
 * Must NOT be confused with the Anthropic ceiling gauge.
 */
export function CapMeter({ measuredValue, budgetValue, budgetBasis, label }: CapMeterProps) {
  const fraction = budgetValue > 0 ? measuredValue / budgetValue : 0;
  const clampedFraction = Math.min(1, Math.max(0, fraction));
  const overCap = fraction >= 1;
  const color = capBarColor(fraction);
  const readout = formatReadout(measuredValue, budgetValue, budgetBasis);

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 3,
        marginTop: 3,
      }}
    >
      {/* Track + fill — thinner than LimitGauge (3px vs 4/6px) */}
      <div
        role="progressbar"
        aria-valuenow={Math.round(clampedFraction * 100)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={`${label} presupuesto`}
        style={{
          position: "relative",
          height: 3,
          borderRadius: "var(--radius-sm)",
          background: "var(--track)",
        }}
      >
        <div
          style={{
            position: "absolute",
            inset: "0 auto 0 0",
            width: `${clampedFraction * 100}%`,
            background: color,
            borderRadius: "var(--radius-sm)",
          }}
        />
      </div>

      {/* Readout with honest unit */}
      <span
        style={{
          fontSize: 10,
          color: overCap ? "var(--gauge-danger)" : "var(--text-muted)",
          fontVariantNumeric: "tabular-nums",
          lineHeight: 1,
        }}
      >
        {readout}
      </span>
    </div>
  );
}
