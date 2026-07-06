import { useWhatsNew } from "./useWhatsNew";
import type { UseWhatsNewResult } from "./useWhatsNew";
import { Markdown } from "@/components/Markdown/Markdown";

// ── View ──────────────────────────────────────────────────────────────────────

type WhatsNewModalViewProps = Pick<
  UseWhatsNewResult,
  "show" | "version" | "versionSection" | "dismiss"
>;

/**
 * Pure presentational What's New modal.
 * Renders the version section as markdown (headings, lists, emphasis, links).
 * Renders nothing when `show` is false.
 */
export function WhatsNewModalView({
  show,
  version,
  versionSection,
  dismiss,
}: WhatsNewModalViewProps) {
  if (!show) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={`Novedades de v${version ?? ""}`}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 1000,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "rgba(0,0,0,0.5)",
      }}
    >
      <div
        style={{
          background: "var(--surface, #fff)",
          border: "1px solid var(--border)",
          borderRadius: "var(--radius-sm)",
          padding: "var(--space-md)",
          minWidth: 320,
          maxWidth: 540,
          maxHeight: "80vh",
          overflowY: "auto",
          display: "flex",
          flexDirection: "column",
          gap: "var(--space-md)",
        }}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "var(--space-xs)",
          }}
        >
          <h2
            style={{
              margin: 0,
              fontSize: 16,
              fontWeight: 700,
              color: "var(--text)",
            }}
          >
            Novedades
            {version ? (
              <span
                style={{
                  marginLeft: 8,
                  fontWeight: 400,
                  fontSize: 13,
                  color: "var(--text-muted)",
                }}
              >
                v{version}
              </span>
            ) : null}
          </h2>
          <button
            onClick={dismiss}
            aria-label="Cerrar"
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              fontSize: 18,
              lineHeight: 1,
              color: "var(--text-muted)",
              padding: "2px 6px",
            }}
          >
            ×
          </button>
        </div>

        {/* Body */}
        <div
          style={{
            fontSize: 13,
            color: "var(--text)",
            fontFamily: "inherit",
          }}
        >
          {versionSection ? (
            <Markdown source={versionSection} />
          ) : (
            "Sin notas para esta versión."
          )}
        </div>

        {/* Footer */}
        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <button
            onClick={dismiss}
            style={{
              background: "var(--accent)",
              border: "none",
              borderRadius: "var(--radius-sm)",
              padding: "4px 16px",
              cursor: "pointer",
              fontSize: 12,
              color: "#fff",
              fontWeight: 600,
            }}
          >
            Entendido
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Connected ─────────────────────────────────────────────────────────────────

/**
 * Connected What's New modal: wires `useWhatsNew` and renders the view.
 * Mount this once in the main window so the modal appears on version change.
 */
export function WhatsNewModal() {
  const state = useWhatsNew();
  return <WhatsNewModalView {...state} />;
}
