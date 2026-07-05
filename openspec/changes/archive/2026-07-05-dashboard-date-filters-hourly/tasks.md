## 1. Backend — query_series (usage/mod.rs)

- [x] 1.1 Agregar la variante `Bucket::Hour` al enum `Bucket` (con su `serde` rename a `"hour"`).
- [x] 1.2 En `query_series_inner`, mapear el formato strftime por bucket incluyendo `Hour → "%Y-%m-%d %H:00"` y aplicar el modificador `'localtime'` a `strftime(<fmt>, timestamp, 'localtime')` para todos los buckets.
- [x] 1.3 Cambiar el filtro `since`/`until` para comparar contra los valores datetime recibidos directamente (`timestamp >= '<since>'` / `timestamp <= '<until>'`), eliminando la concatenación de día fijo `T00:00:00Z`/`T23:59:59Z`.
- [x] 1.4 Actualizar/añadir tests unitarios en Rust: bucketing por hora, agrupación en hora local, y filtro por rango datetime (últimas 24h).

## 2. Frontend — tipos y contrato (features/usage/types.ts)

- [x] 2.1 Extender `type Bucket` para incluir `"hour"`.
- [x] 2.2 Confirmar/ajustar `SeriesQuery` (`since?`/`until?` ahora datetime ISO UTC) y documentar el formato en el comentario.
- [x] 2.3 Agregar tipos para el rango de fechas: `DateRangePreset` (`"24h" | "3d" | "7d" | "30d" | "month" | "all" | "custom"`) y una estructura de estado de filtros.

## 3. Frontend — lógica de rangos y presets (nuevo util)

- [x] 3.1 Crear un util (p.ej. `dateRange.ts`) que, dado un preset y "ahora" local, devuelva `{ since, until }` en UTC ISO y el `defaultBucket` sugerido (tabla D3 del design).
- [x] 3.2 Implementar la resolución de "Custom" a partir de dos fechas locales acotadas por `earliestDate`/`latestDate` de `usageMeta`.
- [x] 3.3 Implementar el guardarraíl de cardinalidad: helper que determina si `hour` está permitido para el rango activo (umbral ~72h) y degrada a `day` cuando no.
- [x] 3.4 Tests unitarios del util: cada preset produce el rango esperado, el guardarraíl deshabilita `hour` en rangos largos.

## 4. Frontend — controles (features/usage/ChartControls.tsx)

- [x] 4.1 Agregar la opción `{ value: "hour", label: "Hora" }` a `BUCKET_OPTIONS`.
- [x] 4.2 Agregar un control de presets de fecha (`24h · 3d · 7d · 30d · Este mes · Todo · Custom`) y, para "Custom", dos selectores de fecha acotados por los límites de datos.
- [x] 4.3 Al cambiar de preset, setear la granularidad sugerida (override permitido) y deshabilitar visualmente `Hora` cuando el guardarraíl lo indique.
- [x] 4.4 Extender `ChartControlsValue`/props para incluir preset y rango, manteniendo el patrón "fully controlled".

## 5. Frontend — estado y wiring (app/App.tsx, useUsageSeries.ts)

- [x] 5.1 Extender el estado de `App.tsx` para incluir preset + rango resuelto y recomputar `since`/`until` en cada fetch (presets relativos frescos).
- [x] 5.2 Pasar `since`/`until` al `SeriesQuery` que consume `useUsageSeries`.
- [x] 5.3 Ajustar KPIs/rango mostrado para reflejar el rango filtrado en lugar de todo el histórico.

## 6. Frontend — presentación temporal (UsageChart.tsx, UsageTable.tsx)

- [x] 6.1 Formatear las etiquetas del eje X para granularidad por hora (mostrar hora local `HH:00`, con fecha cuando corresponda) y ralear ticks si son muchos.
- [x] 6.2 Ajustar los encabezados de columna de `UsageTable` para etiquetas horarias.

## 7. Verificación

- [x] 7.1 `pnpm typecheck && pnpm lint && pnpm test:run` en verde.
- [x] 7.2 `cargo fmt`, `cargo clippy` y `cargo test` en verde.
- [ ] 7.3 QA manual: cada preset filtra correctamente; bucket por hora muestra actividad en la hora local correcta; guardarraíl deshabilita "Hora" en rangos largos; estado vacío para rangos sin datos.
