/**
 * Full-changelog modal. Renders the packaged changelog as formatted markdown
 * using the Markdown component (headings, lists, emphasis, links).
 */

import { Markdown } from "@/components/Markdown/Markdown";

interface ChangelogModalProps {
  changelogText: string;
  onClose: () => void;
}

export function ChangelogModal({ changelogText, onClose }: ChangelogModalProps) {
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Changelog completo"
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
          minWidth: 340,
          maxWidth: 600,
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
            Changelog
          </h2>
          <button
            onClick={onClose}
            aria-label="Cerrar changelog"
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

        {/* Content */}
        <Markdown source={changelogText} />

        {/* Footer */}
        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <button
            onClick={onClose}
            style={{
              background: "none",
              border: "1px solid var(--border)",
              borderRadius: "var(--radius-sm)",
              padding: "4px 14px",
              cursor: "pointer",
              fontSize: 12,
              color: "var(--text-muted)",
            }}
          >
            Cerrar
          </button>
        </div>
      </div>
    </div>
  );
}
