import { formatCost } from "@/features/usage/format";
import { CapMeter } from "./CapMeter";
import type { GroupBudgetRow, GroupBudgetsSnapshot } from "./types";

interface GroupBudgetRowViewProps {
  row: GroupBudgetRow;
  isSession: boolean;
}

function GroupBudgetRowView({ row, isSession }: GroupBudgetRowViewProps) {
  // A share cap meter requires sessionWeightedPct in session mode; usd uses measuredValue directly.
  const hasShareCap =
    row.budgetBasis === "share" &&
    row.budgetValue !== null &&
    isSession &&
    row.sessionWeightedPct !== null;
  const hasUsdCap =
    row.budgetBasis === "usd" && row.budgetValue !== null && row.measuredValue !== null;
  const hasCap = hasShareCap || hasUsdCap;

  // Secondary text: session → weighted estimate "% sesión", rolling → pure local share
  const secondaryText =
    isSession && row.sessionWeightedPct !== null
      ? `~${Math.round(row.sessionWeightedPct)}% sesión`
      : `${Math.round(row.localCostSharePct)}% costo local`;

  // For share cap meter: use sessionWeightedPct as the measured value
  const capMeasuredValue =
    row.budgetBasis === "share" ? row.sessionWeightedPct! : row.measuredValue!;

  return (
    <div
      style={{
        padding: "3px 0",
        borderBottom: "1px solid var(--hairline)",
      }}
    >
      {/* Name + cost row */}
      <div
        style={{
          display: "flex",
          alignItems: "baseline",
          justifyContent: "space-between",
        }}
      >
        <span
          style={{
            fontSize: 12,
            color: "var(--text)",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            maxWidth: "60%",
          }}
          title={row.name}
        >
          {row.name}
        </span>
        <span
          style={{
            display: "flex",
            gap: "var(--space-xs)",
            fontSize: 12,
            fontVariantNumeric: "tabular-nums",
            flexShrink: 0,
          }}
        >
          <span style={{ color: "var(--text)" }}>{formatCost(row.windowCostUsd)}</span>
          <span style={{ color: "var(--text-subtle)" }}>{secondaryText}</span>
        </span>
      </div>

      {/* CapMeter when there's a cap (share only in session mode) */}
      {hasCap && (
        <CapMeter
          measuredValue={capMeasuredValue}
          budgetValue={row.budgetValue!}
          budgetBasis={row.budgetBasis!}
          label={row.name}
        />
      )}
    </div>
  );
}

interface GroupBudgetsSectionProps {
  snapshot: GroupBudgetsSnapshot | null;
  loading?: boolean;
}

/**
 * Section displaying cost per group for the current 5h window.
 * Follows TodayByProjectList grammar: name left (ellipsis), formatCost(windowCostUsd) as
 * leading value, secondary muted text depends on origin:
 *   session → "~N% sesión" (session-weighted estimate)
 *   rolling → "N% costo local" (pure local share)
 * CapMeter when a cap is defined (share cap only available in session mode).
 * "otros" row last. Omitted entirely when no groups are defined.
 * Does NOT use LimitGauge and does NOT present data as Anthropic session %.
 */
export function GroupBudgetsSection({ snapshot, loading = false }: GroupBudgetsSectionProps) {

  // Loading without any prior data: show placeholder
  if (loading && !snapshot) {
    return (
      <section aria-label="Uso por grupo">
        <h2
          style={{
            margin: "0 0 var(--space-xs)",
            fontSize: 11,
            fontWeight: 700,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: "var(--text-muted)",
          }}
        >
          Uso por grupo
        </h2>
        <p
          role="status"
          style={{ margin: 0, fontSize: 12, color: "var(--text-muted)" }}
        >
          Cargando…
        </p>
      </section>
    );
  }

  // No snapshot (and not loading) → no groups or no data yet: omit section entirely
  if (!snapshot) {
    return null;
  }

  // No defined groups (all rows would be "otros" only): omit section
  // A snapshot with only the "otros" row (groupId === null) means no groups exist
  const definedGroupRows = snapshot.rows.filter((r) => r.groupId !== null);
  if (definedGroupRows.length === 0) {
    return null;
  }

  // Total cost = 0 → empty state
  const totalCost = snapshot.rows.reduce((sum, r) => sum + r.windowCostUsd, 0);

  // Separate "otros" row (groupId === null) to always place it last
  const namedRows = snapshot.rows.filter((r) => r.groupId !== null);
  const otrosRow = snapshot.rows.find((r) => r.groupId === null) ?? null;

  const isSession = snapshot.origin === "session";

  return (
    <section aria-label="Uso por grupo">
      {/* Section header */}
      <h2
        style={{
          margin: "0 0 2px",
          fontSize: 11,
          fontWeight: 700,
          textTransform: "uppercase",
          letterSpacing: "0.06em",
          color: "var(--text-muted)",
        }}
      >
        Uso por grupo
      </h2>

      {/* Subtitle / caption */}
      <p
        style={{
          margin: "0 0 var(--space-xs)",
          fontSize: 10,
          color: "var(--text-subtle)",
          lineHeight: 1.2,
        }}
      >
        {isSession
          ? "estimado sobre tu sesión de 5h"
          : "costo local · ventana móvil de 5h (sin sesión activa)"}
      </p>

      {/* Empty state: groups exist but no activity this window */}
      {totalCost === 0 ? (
        <p
          role="status"
          style={{
            margin: 0,
            fontSize: 12,
            color: "var(--text-muted)",
            fontStyle: "italic",
          }}
        >
          Sin consumo en esta ventana de 5h
        </p>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {namedRows.map((row) => (
            <GroupBudgetRowView key={row.groupId} row={row} isSession={isSession} />
          ))}
          {otrosRow !== null && (
            <GroupBudgetRowView key="otros" row={otrosRow} isSession={isSession} />
          )}
        </div>
      )}
    </section>
  );
}
