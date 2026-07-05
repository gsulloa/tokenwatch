## Why

Hoy el dashboard de uso solo permite agrupar por día/semana/mes y siempre grafica todo el histórico: no hay forma de acotar el rango de fechas ni de ver el detalle intra-día. Para diagnosticar picos de consumo o revisar "qué pasó en las últimas horas" se necesita un filtro de fecha con presets rápidos y una granularidad por hora.

## What Changes

- Se agrega un **control de rango de fechas** con presets recomendados: `Últimas 24h`, `Últimos 3 días`, `Últimos 7 días`, `Últimos 30 días`, `Este mes`, `Todo` y `Custom` (dos selectores de fecha acotados por el rango real de datos).
- Se agrega **granularidad por hora** (`hour`) a los buckets existentes (queda `Hora · Día · Semana · Mes`).
- Todos los buckets temporales y presets se **muestran en hora local**, mientras que los datos se siguen **almacenando y filtrando en UTC**. El bucketing horario/diario se calcula en hora local (maneja DST); si la zona local del usuario cambia, el mismo dato UTC se re-agrupa a la nueva hora local sin migración.
- El backend `query_series` amplía la precisión de `since`/`until` de día a **datetime** para poder expresar rangos relativos exactos (p. ej. "últimas 24h"). **BREAKING** en el contrato interno del comando (formato de `since`/`until`), sin consumidores externos.
- El preset seleccionado **sugiere una granularidad por defecto** (24h→hora, 30d→día, etc.), que el usuario puede sobreescribir.
- **Guardarraíl de cardinalidad**: la granularidad `hora` se deshabilita cuando el rango excede ~3 días para evitar cientos de barras ilegibles.

## Capabilities

### New Capabilities
<!-- Ninguna capability nueva; esto extiende el gráfico de uso existente. -->

### Modified Capabilities
- `usage-charts`: `query_series` acepta granularidad `hour` y filtros `since`/`until` con precisión datetime; el bucketing temporal se agrupa en hora local. El dashboard suma un control de rango de fechas con presets y expone la granularidad por hora con su guardarraíl.

## Impact

- **Backend** (`packages/app/src-tauri/src/usage/mod.rs`): variante `Bucket::Hour`, `strftime(..., 'localtime')` en el bucketing, `since`/`until` con precisión datetime en el `WHERE`.
- **Frontend** (`packages/app/src/features/usage/`): nuevo control de presets de fecha + estado en `App.tsx`; opción `Hora` y lógica preset→granularidad + guardarraíl en `ChartControls.tsx`; tipos `Bucket`/`SeriesQuery` en `types.ts`; formato de etiquetas horarias en el eje X (`UsageChart.tsx`) y encabezados de la tabla (`UsageTable.tsx`).
- **Datos**: sin cambios de esquema en SQLite; `usage_meta` (ya existente) provee los límites para el rango "Custom".
