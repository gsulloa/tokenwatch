## Context

El panel de uso (`packages/app/src/features/usage/`) renderiza hoy un `LineChart` de Recharts (`UsageChart.tsx`). La lógica de agrupación top-N/"Otros" vive **solo en el frontend** (`processSeriesResponse`, `TOP_N = 8`); el backend `query_series` ya devuelve **todas** las series con puntos alineados a `buckets` (huecos rellenados con 0). El estilo usa **CSS variables** en `styles/global.css` (no Tailwind), con tokens de color, radios y espaciado y soporte de tema claro/oscuro. La paleta de series (`colors.ts`) tiene 10 colores asignados por hash del nombre.

El grueso del cambio es de frontend (visualización + dashboard), pero también toca backend: la regla de derivación de `project_name` (Rust `ingest`) y sus migraciones de backfill (`db`), más la config de ventana (`tauri.conf.json`).

## Goals / Non-Goals

**Goals:**
- Gráfico de área apilada donde cada punto de X muestra el acumulado de todas las series.
- Mostrar **todas** las series sin agrupar en "Otros".
- Tabla debajo del gráfico con cifras exactas (serie × bucket) y totales por fila y columna.
- Vista de dashboard (topbar, tarjetas KPI, toolbar, paneles) coherente con `/frontend-design` y `/ui-ux-pro-max`: jerarquía, tooltip enriquecido, leyenda, hover coordinado, tema claro/oscuro.
- Ventana de tamaño dashboard, redimensionable y con pantalla completa.
- Paleta determinista escalable a un número arbitrario de series.
- Agrupar los worktrees de agente bajo `unknown` sin perder los proyectos de Conductor, con backfill de datos existentes.

**Non-Goals:**
- Filtrado/selección interactiva de series (más allá de un posible toggle de leyenda opcional).
- Export CSV/PNG (queda para futuro).
- Paginación o virtualización avanzada de la tabla.
- Cambios en el esquema de columnas de SQLite (solo backfill de valores).

## Decisions

### 1. Área apilada (`AreaChart` + `stackId`) en vez de líneas
Recharts `AreaChart` con cada `<Area stackId="usage">` produce el apilado acumulado por punto en X, exactamente lo pedido ("acumulados por cada punto en x"). El total visual de cada bucket es la suma de sus series.
- **Alternativas:** (a) mantener líneas y sumar aparte → no comunica el acumulado; (b) barras apiladas → válido pero peor para muchos buckets/tendencia temporal; el área apilada conserva la lectura temporal del line chart actual y añade el acumulado. Área apilada gana.
- Relleno con opacidad moderada (~0.75) y borde de 1.5px del mismo color para separar bandas; `type="monotone"` como hoy.

### 2. Eliminar top-N / "Otros"
Se borra `processSeriesResponse`'s grouping, `TOP_N`, `OTROS_LABEL` y las notas de agrupación. `buildRows` mapea `buckets → { bucket, [serie]: valor }` para **todas** las series, ordenadas por total descendente (para orden de apilado y de tabla estable). El orden de apilado: series mayores abajo, menores arriba (o viceversa) — se fija uno y se documenta para consistencia con la leyenda/tabla.

### 3. Paleta escalable y determinista
Con "Otros" fuera puede haber muchas series. Se amplía `colors.ts` para generar color por índice de orden (no por hash) usando una paleta base ampliada (~16 tonos bien separados) y, si se excede, generación HSL por rotación de tono con saturación/luminancia fijas por tema. Se prioriza asignación por **posición en el orden ordenado** para maximizar contraste entre series adyacentes en el apilado; el mapa nombre→color se calcula una vez por render y se comparte entre gráfico, leyenda y tabla.
- **Alternativa:** seguir con hash → colisiones y colores adyacentes similares con muchas series. Rechazado.

### 4. Tabla de datos (`UsageTable.tsx`)
Nuevo componente que recibe el `SeriesResponse` procesado (mismo orden y mapa de colores que el gráfico). Estructura: primera columna = serie (con chip/punto de color), una columna por bucket, última columna = **Total** por serie; última fila = **Total** por bucket (y gran total en la esquina). Celdas con cifras exactas:
- tokens: separador de miles, sin abreviar K/M (nuevo helper `formatTokensExact`).
- costo: `$` con 2 decimales (reutiliza/afina `formatCost`).
- Encabezado de columnas sticky; scroll horizontal si hay muchos buckets. Alineación numérica a la derecha, fuente monoespaciada (`--font-mono`).
- Tooltip del gráfico enriquecido: valor exacto + % del total del bucket.

### 5. Hover coordinado y leyenda
Al hacer hover en una banda/fila se resalta la serie correspondiente (opacidad del resto reducida) — estado `hoveredSeries` compartido entre `UsageChart` y `UsageTable` elevado a un contenedor (`UsagePanel`/`App`). Leyenda con puntos de color; opcionalmente clic para atenuar (no requerido por spec, se deja como mejora si el tiempo lo permite).

### 6. Layout de dashboard
`App.tsx` compone la vista como dashboard: topbar fija (título + subtítulo, último refresh, botón actualizar) → fila de tarjetas KPI (grid `auto-fit`) → toolbar con `ChartControls` → panel del gráfico (área apilada, ~360px) → panel de tabla. Las KPI se calculan del `SeriesResponse` + `UsageMeta`: total de la métrica, nº de series, nº de buckets, nº de eventos y rango de fechas. Se mantiene la nota de "costo estimado" cuando `metric=cost` y el estado vacío dentro del panel del gráfico. Estilos por clases en `global.css` (`.dashboard*`, `.kpi-*`, `.panel*`, `.toolbar`).

### 7. Ventana (tauri.conf.json)
La ventana pasa de `420×560` fija a `1280×832` con `minWidth/minHeight`, `resizable: true` y `maximizable: true` (habilita el botón de pantalla completa de macOS). Sin cambios de identidad/título.

### 8. Init de tema (bugfix)
Todos los tokens de color en `global.css` están definidos bajo `:root[data-theme="light"|"dark"]`. Nadie fijaba `data-theme`, así que las variables quedaban indefinidas y la UI no se veía (texto invisible en modo oscuro). En `main.tsx` se fija `data-theme` desde `matchMedia('(prefers-color-scheme: dark)')` antes del render y se escucha su cambio para actualizar en caliente.

### 9. Contrato `UsageMeta` (bugfix)
El comando Rust `usage_meta` serializa `earliestDate`/`latestDate` (camelCase), pero el tipo del frontend declaraba `dateRange: {min,max}`, que no existe en la respuesta → `meta.dateRange.min` crasheaba al poblarse `meta`. Se alinea `types.ts` al contrato real (`earliestDate`/`latestDate`) y `App.tsx` los consume.

### 10. Derivación de `project_name` + backfill (backend)
En `ingest::derive_project_name` el orden es: sin cwd → `unknown`; cualquier componente `worktrees` → `unknown` (worktrees de agente efímeros como `.claude/worktrees/agent-XXXX`); `conductor/workspaces/<repo>` → `<repo>` (tub2, argus, backend…); resto → últimos 1–2 segmentos. Los worktrees se agrupan porque no aportan un proyecto identificable, mientras que los workspaces de Conductor sí (son los proyectos reales).
- **Backfill**: como los datos históricos ya están en SQLite, cada cambio de regla añade una migración gateada por `schema_version` que recomputa `project_name` desde `project_path` (helper `backfill_project_names`, idempotente y atómico). Al iterar sobre la regla se agregaron migraciones sucesivas (v2→repo, v3, v4→worktrees/unknown + repo) para reparar bases que ya habían corrido una versión intermedia.
- **Alternativa:** atribuir los `.claude/worktrees/agent-XXXX` al repo padre (`tub2`). Rechazado: el usuario pidió explícitamente agruparlos en `unknown`.

## Risks / Trade-offs

- **Muchas series → apilado ilegible / tabla ancha** → Mitigación: orden por total (las relevantes destacan), colores de alto contraste, tabla con scroll horizontal y totales; el detalle fino se lee en la tabla, no en el gráfico.
- **Colores insuficientes / poco distinguibles** → Mitigación: paleta ampliada + generación HSL determinista; verificación en tema claro y oscuro.
- **Rendimiento con muchos buckets × series en la tabla** → Mitigación: dataset acotado (buckets por rango visible); si crece, considerar virtualización (no-goal por ahora), documentado.
- **Cambio de line→área altera expectativa visual** → Mitigación: se mantiene interpolación monotone y ejes; el acumulado es el comportamiento pedido explícitamente.
- **Regresión de tests/snapshots existentes de `UsageChart`** → Mitigación: actualizar tests a la nueva estructura (área apilada, sin "Otros") y añadir tests de la tabla.

## Migration Plan

1. Refactor `UsageChart.tsx` a área apilada, sin top-N.
2. Ampliar `colors.ts` (paleta + generador) y compartir mapa de colores.
3. Añadir `formatTokensExact` y `formatPercent` en `format.ts`.
4. Crear `UsageTable.tsx` y `seriesUtils.ts`.
5. Rediseñar `App.tsx` como dashboard (topbar, KPIs, toolbar, paneles) + hover coordinado.
6. Ajustar `styles/global.css` (tabla, KPIs, paneles, toolbar, chips).
7. Agrandar la ventana en `tauri.conf.json` (redimensionable + fullscreen).
8. Init de `data-theme` en `main.tsx`; alinear `UsageMeta` en `types.ts`.
9. Regla de worktrees en `ingest/mod.rs` + migración de backfill en `db/mod.rs`.
10. Actualizar/crear tests: `pnpm typecheck && pnpm lint && pnpm test:run` (frontend) y `cargo fmt/clippy/test` (backend).

Rollback: revertir el commit restaura el line chart con "Otros", la ventana pequeña y la regla previa de `project_name`. Las migraciones de backfill son idempotentes; un rollback de código no revierte los `project_name` ya recomputados en la DB local (se corregirían con la regla activa al reingerir).

## Open Questions

Resueltas durante la implementación:
- Orden de apilado: **mayor abajo** (base estable); leyenda/tabla comparten el mismo orden.
- Formato de la tabla: **siempre exacto** (miles con separador para tokens, `$` con 2 decimales para costo).
- Worktrees de agente: se agrupan en **`unknown`** (no se atribuyen al repo padre).
