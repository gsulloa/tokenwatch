import { LimitGauge } from "./LimitGauge";
import type { LimitsSnapshot } from "./types";

interface UnavailableMessageProps {
  reason: string;
}

function UnavailableMessage({ reason }: UnavailableMessageProps) {
  let message: string;

  switch (reason) {
    case "not_signed_in":
    case "expired":
      message = "Abre Claude Code para actualizar el uso";
      break;
    case "keychain_denied":
      message =
        'Permiso denegado al Keychain. Abre las preferencias de seguridad y elige "Permitir siempre" para TokenWatch.';
      break;
    case "network":
    case "http":
    case "parse":
    default:
      message = "No se pudo obtener el uso";
      break;
  }

  return (
    <p
      role="status"
      style={{
        margin: 0,
        fontSize: 12,
        color: "var(--text-muted)",
        fontStyle: "italic",
      }}
    >
      {message}
    </p>
  );
}

interface LimitsSectionProps {
  snapshot: LimitsSnapshot | null;
  loading?: boolean;
}

/**
 * Renders session (5h) and weekly limit gauges, plus per-model sub-section.
 * Shows an explicit message when limits are unavailable.
 */
export function LimitsSection({ snapshot, loading = false }: LimitsSectionProps) {
  if (loading && !snapshot) {
    return (
      <p
        role="status"
        style={{ margin: 0, fontSize: 12, color: "var(--text-muted)" }}
      >
        Cargando límites…
      </p>
    );
  }

  if (!snapshot) {
    return (
      <p
        role="status"
        style={{ margin: 0, fontSize: 12, color: "var(--text-muted)" }}
      >
        Sin datos de límites
      </p>
    );
  }

  const isUnavailable = snapshot.status.kind !== "ok";
  const reason =
    isUnavailable && snapshot.status.kind === "unavailable"
      ? snapshot.status.reason
      : "";

  return (
    <section aria-label="Límites de uso">
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
        Límites
      </h2>

      {isUnavailable ? (
        <UnavailableMessage reason={reason} />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
          {snapshot.session && (
            <LimitGauge
              label="Sesión 5h"
              utilization={snapshot.session.utilization}
              resetsAt={snapshot.session.resetsAt}
            />
          )}

          {snapshot.weekly && (
            <LimitGauge
              label="Semana"
              utilization={snapshot.weekly.utilization}
              resetsAt={snapshot.weekly.resetsAt}
            />
          )}

          {!snapshot.session && !snapshot.weekly && (
            <p
              style={{ margin: 0, fontSize: 12, color: "var(--text-muted)" }}
            >
              Sin datos de límites disponibles
            </p>
          )}

          {snapshot.weeklyByModel.length > 0 && (
            <div style={{ marginTop: "var(--space-xs)" }}>
              <h3
                style={{
                  margin: "0 0 var(--space-xs)",
                  fontSize: 10,
                  fontWeight: 700,
                  textTransform: "uppercase",
                  letterSpacing: "0.06em",
                  color: "var(--text-subtle)",
                }}
              >
                Por modelo
              </h3>
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: "var(--space-xs)",
                }}
              >
                {snapshot.weeklyByModel.map((win, i) => (
                  <LimitGauge
                    key={win.label ?? String(i)}
                    label={win.label ?? "Modelo desconocido"}
                    utilization={win.utilization}
                    resetsAt={win.resetsAt}
                    compact
                  />
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </section>
  );
}
