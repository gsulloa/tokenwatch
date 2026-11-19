import { formatResetGrantsLabel, summarizeResetGrants } from "./resetGrants";
import type { ResetGrantSummary } from "./types";

interface ResetGrantsRowProps {
  grants: ResetGrantSummary[];
}

/** Compact informational row: free limit resets available to claim in claude.ai. */
export function ResetGrantsRow({ grants }: ResetGrantsRowProps) {
  const summary = summarizeResetGrants(grants);
  if (!summary) return null;

  return (
    <div
      role="status"
      aria-label="Reinicios de límites"
      title="Reclámalo en claude.ai → Configuración → Uso"
      style={{
        display: "flex",
        alignItems: "center",
        gap: "var(--space-xs)",
        fontSize: 12,
        color: "var(--text-muted)",
      }}
    >
      <span
        aria-hidden
        style={{
          width: 6,
          height: 6,
          borderRadius: "var(--radius-full)",
          background: "var(--info)",
          flexShrink: 0,
        }}
      />
      <span
        style={{
          fontFamily: "var(--font-data)",
          fontVariantNumeric: "tabular-nums",
        }}
      >
        {formatResetGrantsLabel(summary)}
      </span>
    </div>
  );
}
