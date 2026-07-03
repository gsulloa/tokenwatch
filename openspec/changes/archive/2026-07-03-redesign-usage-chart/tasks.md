## 1. Colores y formateo

- [x] 1.1 Ampliar `colors.ts`: paleta base de ~16 tonos de alto contraste + generador HSL determinista para cuando se excede, y asignación por índice de orden (no por hash). Exponer un helper que reciba la lista ordenada de nombres y devuelva un `Map<string,string>` nombre→color estable.
- [x] 1.2 En `format.ts`: añadir `formatTokensExact` (miles con separador, sin K/M) y un helper de porcentaje (`formatPercent` o similar) para el tooltip. Mantener `formatCost`.

## 2. Gráfico de área apilada

- [x] 2.1 En `UsageChart.tsx`: eliminar `TOP_N`, `OTROS_LABEL` y la agrupación en `processSeriesResponse`; construir filas para TODAS las series ordenadas por total descendente.
- [x] 2.2 Reemplazar `LineChart`/`Line` por `AreaChart`/`Area` con `stackId` común (apilado acumulado por punto en X), `type="monotone"`, relleno ~0.75 y borde 1.5px del color de serie.
- [x] 2.3 Enriquecer el `Tooltip`: mostrar valor exacto por serie + % del total del bucket; leyenda con puntos de color; mantener nota de "costo estimado" cuando `metric=cost` y el estado vacío.
- [x] 2.4 Compartir el mapa de colores (1.1) entre gráfico y tabla; aceptar `hoveredSeries` para atenuar las bandas no activas.

## 3. Tabla de datos

- [x] 3.1 Crear `UsageTable.tsx`: fila por serie (con chip/punto de color), columna por bucket, columna final "Total" por serie, fila final "Total" por bucket y gran total en la esquina.
- [x] 3.2 Formatear celdas con cifras exactas según métrica (tokens con `formatTokensExact`, costo con `formatCost`); alineación numérica a la derecha, `--font-mono`, encabezados sticky y scroll horizontal.
- [x] 3.3 Usar el mismo orden y mapa de colores del gráfico; emitir/recibir `hoveredSeries` para resaltar la fila correspondiente.

## 4. Layout y rediseño

- [x] 4.1 En `App.tsx` (o nuevo `UsagePanel`): componer controles → gráfico → tabla, y elevar el estado `hoveredSeries` compartido.
- [x] 4.2 Ajustar `styles/global.css`: estilos de tabla, chips de color, jerarquía tipográfica y verificación en tema claro y oscuro (`/frontend-design`, `/ui-ux-pro-max`).

## 5. Dashboard y ventana

- [x] 5.1 Rediseñar `App.tsx` como dashboard: topbar fija (título/subtítulo, último refresh, botón actualizar), fila de tarjetas KPI (total métrica, series, períodos, eventos, rango de fechas), toolbar de controles, y paneles tipo card para gráfico y tabla.
- [x] 5.2 Estilos de dashboard en `global.css` (`.dashboard*`, `.kpi-*`, `.panel*`, `.toolbar`) usando tokens; verificación en tema claro y oscuro.
- [x] 5.3 Agrandar la ventana en `tauri.conf.json` (1280×832, `minWidth/minHeight`, `resizable`, `maximizable`).

## 6. Bugfixes de render

- [x] 6.1 Inicializar `data-theme` en `main.tsx` según `prefers-color-scheme` + listener de cambio (sin esto los tokens CSS quedaban indefinidos y la UI no se veía).
- [x] 6.2 Alinear `UsageMeta` (`types.ts`) con el contrato Rust (`earliestDate`/`latestDate`) y consumir esos campos en `App.tsx` (arregla crash `meta.dateRange.min`).

## 7. Backend: agrupación de worktrees

- [x] 7.1 En `ingest::derive_project_name`: cualquier componente `worktrees` → `unknown`; `conductor/workspaces/<repo>` → `<repo>`; resto → últimos 1–2 segmentos. Tests de derivación.
- [x] 7.2 Migración de backfill en `db/mod.rs` (gateada por `schema_version`, helper `backfill_project_names` idempotente) que recomputa `project_name` de las filas existentes; tests de backfill.

## 8. Verificación

- [x] 8.1 Actualizar/crear tests frontend: `UsageChart` (área apilada, sin "Otros", acumulado por bucket) y `UsageTable` (totales por fila/columna, cifras exactas, consistencia de color/orden).
- [x] 8.2 Ejecutar `pnpm typecheck && pnpm lint && pnpm test:run` (frontend) y `cargo fmt/clippy/test` (backend); verificar visualmente el dashboard en la app (claro y oscuro).
