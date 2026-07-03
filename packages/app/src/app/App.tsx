import { useState } from "react";
import { APP_DISPLAY_NAME } from "@/platform/app-identity";
import { ChartControls } from "@/features/usage/ChartControls";
import { UsageChart } from "@/features/usage/UsageChart";
import { useUsageSeries } from "@/features/usage/useUsageSeries";
import type { ChartControlsValue } from "@/features/usage/ChartControls";

const DEFAULT_CONTROLS: ChartControlsValue = {
  bucket: "day",
  metric: "tokens",
  seriesBy: "model",
};

function formatRefreshAt(iso: string | null | undefined): string {
  if (!iso) return "Nunca";
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

export function App() {
  const [controls, setControls] =
    useState<ChartControlsValue>(DEFAULT_CONTROLS);

  const { data, meta, loading, error, refresh } = useUsageSeries({
    bucket: controls.bucket,
    metric: controls.metric,
    seriesBy: controls.seriesBy,
  });

  return (
    <main style={{ padding: 24, fontFamily: "var(--font-stack)" }}>
      <div
        style={{
          display: "flex",
          alignItems: "baseline",
          justifyContent: "space-between",
          marginBottom: "var(--space-md)",
        }}
      >
        <h1 style={{ margin: 0 }}>{APP_DISPLAY_NAME}</h1>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
            Actualizado: {formatRefreshAt(meta?.lastRefreshAt)}
          </span>
          <button
            onClick={refresh}
            disabled={loading}
            style={{ fontSize: 11, padding: "3px 8px" }}
            aria-label="Actualizar datos"
          >
            {loading ? "Cargando…" : "Actualizar"}
          </button>
        </div>
      </div>

      <div style={{ marginBottom: "var(--space-md)" }}>
        <ChartControls value={controls} onChange={setControls} />
      </div>

      {error && (
        <p
          role="alert"
          style={{
            color: "var(--danger)",
            fontSize: 12,
            margin: "0 0 var(--space-sm)",
          }}
        >
          Error al cargar datos: {error}
        </p>
      )}

      {data ? (
        <UsageChart response={data} />
      ) : !loading ? (
        <div
          role="status"
          aria-label="Sin datos"
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            height: 240,
            gap: 8,
            color: "var(--text-muted)",
          }}
        >
          <span style={{ fontSize: 32 }}>📊</span>
          <p style={{ margin: 0, fontWeight: 600 }}>Sin datos de uso</p>
          <p style={{ margin: 0, fontSize: 12 }}>
            Conecta con el backend de Tauri para ver datos de uso.
          </p>
        </div>
      ) : null}
    </main>
  );
}
