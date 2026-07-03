## Why

El gráfico de uso actual es un line chart plano que agrupa las series menos usadas en "Otros", ocultando el detalle por proyecto/modelo justo cuando más series hay. Además no ofrece los números exactos y vive en una ventana pequeña de menú, sin una vista de dashboard. El usuario necesita ver **todo el detalle sin agrupar**, entender **el total acumulado en cada punto del eje X**, **leer las cifras exactas** en una tabla, y disponer de una **vista de dashboard** más grande (redimensionable / pantalla completa).

## What Changes

- **BREAKING**: Se elimina la agrupación "Otros" (top-N). Todas las series se muestran individualmente, sin importar cuántas haya, para no perder detalle.
- Se reemplaza el line chart por un **gráfico de área apilada (stacked area)**: en cada punto del eje X las series se acumulan una sobre otra, de modo que la altura total representa el consumo agregado del bucket y cada banda el aporte de cada serie.
- Se agrega una **tabla de datos debajo del gráfico** con una fila por serie y una columna por bucket (más una columna de total por serie y una fila de total por bucket), mostrando las cifras exactas de tokens o costo.
- **Rediseño completo de la vista como dashboard** siguiendo `/frontend-design` y `/ui-ux-pro-max`: barra superior fija, fila de tarjetas KPI de resumen (total, nº de series, nº de períodos, eventos, rango de fechas), toolbar de controles, y paneles tipo card para gráfico y tabla. Tooltip enriquecido (valor + % del total del bucket), leyenda con puntos de color, hover coordinado entre gráfico y tabla, jerarquía tipográfica y buen comportamiento en tema claro/oscuro.
- **Ventana más grande y redimensionable**: la ventana pasa de 420×560 fija a 1280×832 con mínimos, `resizable` y `maximizable`, habilitando pantalla completa.
- La paleta de colores se amplía para soportar un número arbitrario de series de forma determinista (asignación por índice de orden; mismo nombre → mismo color).
- **Agrupación de worktrees en el nombre de proyecto**: los `cwd` que corresponden a worktrees efímeros de agente (cualquier componente `worktrees`, p.ej. `.../.claude/worktrees/agent-XXXX`) se agrupan bajo `unknown`; los workspaces de Conductor (`conductor/workspaces/<repo>/...`) siguen resolviéndose al repositorio (`tub2`, `argus`, `backend`…). Incluye migración de backfill para corregir filas ya almacenadas.
- **Bugfixes descubiertos durante la implementación**:
  - Se inicializa el atributo `data-theme` según `prefers-color-scheme` (sin él, todos los tokens CSS quedaban indefinidos y la UI no se veía).
  - Se alinea el tipo `UsageMeta` del frontend con el contrato real de Rust (`earliestDate`/`latestDate` en vez de `dateRange`), que causaba un crash al poblarse `meta`.

## Capabilities

### New Capabilities
<!-- Ninguna capability nueva; el cambio modifica usage-charts y usage-ingestion. -->

### Modified Capabilities
- `usage-charts`: se **elimina** la agrupación "otros"; se **modifica** la visualización de line chart a área apilada (acumulado por punto en X); se **añaden** requisitos de tabla de cifras exactas, vista de dashboard con KPIs, ventana redimensionable/pantalla completa, y aplicación de tema según la preferencia del sistema.
- `usage-ingestion`: se **modifica** la derivación de `project_name` para agrupar los worktrees de agente bajo `unknown` (manteniendo la resolución al repo para workspaces de Conductor), con backfill que corrige las filas existentes.

## Impact

- **Frontend** (`packages/app/src/`):
  - `features/usage/UsageChart.tsx` — elimina top-N/"Otros"; `LineChart`→`AreaChart`/`Area` apilado (`stackId`); tooltip y leyenda personalizados; hover coordinado.
  - `features/usage/UsageTable.tsx` (nuevo) — tabla serie × buckets con totales.
  - `features/usage/seriesUtils.ts` (nuevo) — `orderSeries` (orden por total desc).
  - `features/usage/colors.ts` — paleta ampliada + generador HSL determinista + `buildColorMap`.
  - `features/usage/format.ts` — `formatTokensExact`, `formatPercent` (además de `formatCost`).
  - `features/usage/types.ts` — `UsageMeta` alineado a Rust (`earliestDate`/`latestDate`).
  - `app/App.tsx` — rediseño de dashboard (topbar, KPIs, toolbar, paneles) y estado `hoveredSeries` compartido.
  - `main.tsx` — init de `data-theme` según `prefers-color-scheme`.
  - `styles/global.css` — estilos de tabla, tarjetas KPI, paneles, toolbar y chips de color.
- **Backend** (`packages/app/src-tauri/`):
  - `src/ingest/mod.rs` — regla de derivación de `project_name` (worktrees→`unknown`; Conductor→repo).
  - `src/db/mod.rs` — migraciones de backfill (`schema_version`) que recomputan `project_name` de las filas existentes.
  - `tauri.conf.json` — tamaño de ventana, `resizable`, `maximizable`, mínimos.
- **Specs**: `usage-charts` (delta) y `usage-ingestion` (delta).
