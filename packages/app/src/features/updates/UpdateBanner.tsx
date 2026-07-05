import type { UseAppUpdateResult } from "./types";
import { useAppUpdate } from "./useAppUpdate";

/**
 * Pure presentational banner driven by hook state.
 * Accepting the hook result as props makes each state independently testable.
 *
 * - `idle`        → renders nothing (null).
 * - `checking`    → shows spinner text, actions disabled.
 * - `available`   → "Actualización disponible → vX.Y.Z" + "Instalar" button.
 * - `downloading` → shows progress, action disabled.
 * - `ready`       → "Reiniciar para aplicar" button.
 * - `error`       → discreet error line (only after manual check).
 *
 * Always includes a "Buscar actualizaciones" affordance (calling `checkNow`).
 * Styling follows the Popover convention: inline styles + CSS vars.
 */
export function UpdateBannerView({
  status,
  version,
  progress,
  error,
  checkNow,
  installNow,
  relaunchApp,
}: UseAppUpdateResult) {
  if (status === "idle") return null;

  const isBusy = status === "checking" || status === "downloading";

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-xs)",
        padding: "var(--space-xs) 0",
      }}
      aria-label="Estado de actualización"
    >
      {status === "checking" && (
        <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
          Buscando actualizaciones…
        </span>
      )}

      {status === "available" && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "var(--space-xs)",
          }}
        >
          <span style={{ fontSize: 12, color: "var(--text)" }}>
            Actualización disponible
            {version ? ` → v${version}` : ""}
          </span>
          <button
            onClick={installNow}
            style={{
              background: "var(--accent)",
              border: "none",
              borderRadius: "var(--radius-sm)",
              padding: "2px 8px",
              cursor: "pointer",
              fontSize: 11,
              color: "#fff",
              fontWeight: 600,
            }}
          >
            Instalar
          </button>
        </div>
      )}

      {status === "downloading" && (
        <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
          Instalando…{progress !== null ? ` ${progress}%` : ""}
        </span>
      )}

      {status === "ready" && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "var(--space-xs)",
          }}
        >
          <span style={{ fontSize: 12, color: "var(--text)" }}>
            Actualización lista
          </span>
          <button
            onClick={relaunchApp}
            style={{
              background: "var(--accent)",
              border: "none",
              borderRadius: "var(--radius-sm)",
              padding: "2px 8px",
              cursor: "pointer",
              fontSize: 11,
              color: "#fff",
              fontWeight: 600,
            }}
          >
            Reiniciar para aplicar
          </button>
        </div>
      )}

      {status === "error" && error && (
        <span
          role="alert"
          style={{ fontSize: 11, color: "var(--danger)", fontStyle: "italic" }}
        >
          {error}
        </span>
      )}

      {/* Manual check affordance — shown in non-busy states */}
      {!isBusy && (
        <button
          onClick={checkNow}
          style={{
            background: "none",
            border: "none",
            padding: 0,
            cursor: "pointer",
            fontSize: 11,
            color: "var(--text-muted)",
            textDecoration: "underline",
            textAlign: "left",
          }}
        >
          Buscar actualizaciones
        </button>
      )}
    </div>
  );
}

/**
 * Connected banner: wires `useAppUpdate` into `UpdateBannerView`.
 * Use this in the Popover.
 */
export function UpdateBanner() {
  const updateState = useAppUpdate();
  return <UpdateBannerView {...updateState} />;
}
