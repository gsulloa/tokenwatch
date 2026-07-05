import { useState } from "react";
import { useAppVersion } from "./useAppVersion";
import { useAppUpdate } from "@/features/updates/useAppUpdate";
import type { UseAppUpdateResult } from "@/features/updates/types";
import { ChangelogModal } from "./ChangelogModal";
import changelogRaw from "@/generated/changelog.md?raw";

// ── View ──────────────────────────────────────────────────────────────────────

export interface AboutSectionViewProps {
  /** Current version string, or null outside Tauri. */
  version: string | null;
  /** Full update state from useAppUpdate. */
  updateState: UseAppUpdateResult;
  /** Called when the user clicks "Ver changelog". */
  onOpenChangelog: () => void;
}

/**
 * Pure presentational "Acerca de / Versión" section.
 * Reuses the update-action semantics from UpdateBanner.
 */
export function AboutSectionView({
  version,
  updateState,
  onOpenChangelog,
}: AboutSectionViewProps) {
  const { status, version: updateVersion, progress, error, checkNow, installNow, relaunchApp } =
    updateState;

  const isBusy = status === "checking" || status === "downloading";

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-xs)",
      }}
    >
      {/* Version row */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "var(--space-xs)",
        }}
      >
        <span
          style={{
            fontSize: 13,
            fontWeight: 600,
            color: "var(--text)",
            fontVariantNumeric: "tabular-nums",
          }}
          aria-label="Versión actual"
        >
          {version !== null ? `v${version}` : "v—"}
        </span>

        <button
          onClick={onOpenChangelog}
          style={{
            background: "none",
            border: "none",
            padding: 0,
            cursor: "pointer",
            fontSize: 11,
            color: "var(--text-muted)",
            textDecoration: "underline",
          }}
        >
          Ver changelog
        </button>
      </div>

      {/* Update status */}
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
            Actualización disponible{updateVersion ? ` → v${updateVersion}` : ""}
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

      {/* Idle / up-to-date row */}
      {status === "idle" && (
        <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
          App al día
        </span>
      )}

      {/* Manual check affordance */}
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

// ── Connected ─────────────────────────────────────────────────────────────────

export interface AboutSectionProps {
  /**
   * Optional override for the "Ver changelog" action. When provided, clicking
   * "Ver changelog" delegates to this callback and NO in-place modal is
   * rendered — used in the small popover window, which delegates to the
   * dashboard window instead of rendering the (clipped) modal itself.
   * When omitted, the section opens the changelog modal in place.
   */
  onOpenChangelog?: () => void;
}

/**
 * Connected "Acerca de" section: wires version + update hooks, manages
 * changelog modal visibility (unless the caller delegates via
 * `onOpenChangelog`).
 */
export function AboutSection({ onOpenChangelog }: AboutSectionProps = {}) {
  const { version } = useAppVersion();
  const updateState = useAppUpdate();
  const [showChangelog, setShowChangelog] = useState(false);

  const delegated = onOpenChangelog !== undefined;

  return (
    <>
      <AboutSectionView
        version={version}
        updateState={updateState}
        onOpenChangelog={onOpenChangelog ?? (() => setShowChangelog(true))}
      />
      {!delegated && showChangelog && (
        <ChangelogModal
          changelogText={changelogRaw}
          onClose={() => setShowChangelog(false)}
        />
      )}
    </>
  );
}
