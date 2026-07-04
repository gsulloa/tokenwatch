import { formatTokens } from "./format";
import type { TodayByProject } from "@/features/limits/types";

interface TodayByProjectListProps {
  data: TodayByProject | null;
  loading?: boolean;
}

/**
 * Ordered list of projects for today, with token counts and % of total.
 * Shows a total-of-day row at the bottom. Empty state when rows is empty.
 */
export function TodayByProjectList({
  data,
  loading = false,
}: TodayByProjectListProps) {
  if (loading && !data) {
    return (
      <section aria-label="Consumo hoy por proyecto">
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
          Hoy por proyecto
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

  const rows = data?.rows ?? [];
  const totalTokens = data?.totalTokens ?? 0;

  return (
    <section aria-label="Consumo hoy por proyecto">
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
        Hoy por proyecto
      </h2>

      {rows.length === 0 ? (
        <p
          role="status"
          style={{
            margin: 0,
            fontSize: 12,
            color: "var(--text-muted)",
            fontStyle: "italic",
          }}
        >
          Sin consumo registrado hoy
        </p>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {rows.map((row) => (
            <div
              key={row.project}
              style={{
                display: "flex",
                alignItems: "baseline",
                justifyContent: "space-between",
                padding: "3px 0",
                borderBottom: "1px solid var(--hairline)",
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
                title={row.project}
              >
                {row.project}
              </span>
              <span
                style={{
                  display: "flex",
                  gap: "var(--space-xs)",
                  fontSize: 12,
                  color: "var(--text-muted)",
                  fontVariantNumeric: "tabular-nums",
                  flexShrink: 0,
                }}
              >
                <span>{formatTokens(row.tokens)}</span>
                <span style={{ color: "var(--text-subtle)" }}>
                  {Math.round(row.pct)}%
                </span>
              </span>
            </div>
          ))}

          {/* Total row */}
          <div
            style={{
              display: "flex",
              alignItems: "baseline",
              justifyContent: "space-between",
              padding: "4px 0 2px",
              marginTop: 2,
            }}
          >
            <span
              style={{
                fontSize: 12,
                fontWeight: 700,
                color: "var(--text)",
              }}
            >
              Total del día
            </span>
            <span
              style={{
                fontSize: 12,
                fontWeight: 700,
                color: "var(--text)",
                fontVariantNumeric: "tabular-nums",
              }}
            >
              {formatTokens(totalTokens)}
            </span>
          </div>
        </div>
      )}
    </section>
  );
}
