## Why

En el dashboard, el gráfico solo dibuja en el eje X los buckets temporales que tienen eventos: los buckets sin actividad desaparecen del eje en lugar de mostrarse en 0. En la vista de "últimas 24 horas" esto significa que solo aparecen las horas con consumo, dejando un eje X irregular y engañoso (parece que solo existieron esas horas). El usuario quiere ver el rango completo —p.ej. las 24 horas— con los buckets vacíos representados en 0.

El origen es que `query_series` deriva la lista de buckets a partir de las filas que devuelve el `GROUP BY` de SQLite (solo buckets con datos), en vez de generar todos los buckets que caben en el rango solicitado. El relleno con 0 actual solo cubre huecos *entre* buckets existentes, no el rango completo.

## What Changes

- `query_series` genera la lista completa de buckets que caben en el rango efectivo (`since`/`until`) según la granularidad activa (hora/día/semana/mes), en hora local, y no solo los buckets presentes en los datos.
- Cada serie se alinea contra esa lista completa; los buckets sin eventos se devuelven en 0.
- Cuando no se especifica `since`/`until`, el rango se deriva del min/max real de los eventos (comportamiento actual) pero se rellenan todos los buckets intermedios.
- El gráfico y la tabla del dashboard reflejan automáticamente los buckets vacíos (0) porque consumen `buckets` + `series` del backend; no requieren cambios de contrato.

## Capabilities

### New Capabilities
<!-- Ninguna -->

### Modified Capabilities
- `usage-charts`: se refuerza el requisito "Consulta de series temporales agregadas" para que la respuesta incluya **todos** los buckets del rango solicitado (no solo los que tienen eventos), rellenando en 0 los buckets sin actividad a lo largo de todo el rango.

## Impact

- Backend Rust: `packages/app/src-tauri/src/usage/mod.rs` — función `query_series_inner()` (construcción de la lista de buckets y relleno de series). Requiere lógica para enumerar buckets por granularidad en hora local.
- Tests Rust: se actualiza/añade cobertura (`test_empty_bucket_filling` y casos de rango con horas vacías).
- Frontend: sin cambios de contrato; `UsageChart.tsx` y la tabla renderizan los buckets adicionales automáticamente. Posible ajuste menor en densidad de ticks del eje X si el número de buckets crece (ya cubierto por `computeTickInterval`).
