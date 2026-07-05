## Context

El dashboard de uso consulta `query_series` (Tauri, `usage/mod.rs`) que ya acepta `since`/`until` opcionales pero el frontend nunca los envía. El bucketing usa `strftime('<fmt>', timestamp)` sobre timestamps almacenados en UTC (RFC3339). Los buckets soportados son `day`/`week`/`month`. El filtro de fecha actual compara strings con precisión de día en UTC (`timestamp >= '<since>T00:00:00Z'`). `query_today_by_project` ya demuestra el patrón de trabajar en hora local (convierte local→UTC en Rust vía chrono). `usage_meta` ya devuelve `earliest_date`/`latest_date`.

Este cambio suma un filtro de rango de fechas con presets, granularidad por hora, y traslada el bucketing a hora local, manteniendo el almacenamiento en UTC.

## Goals / Non-Goals

**Goals:**
- Filtro de rango de fechas con presets (`24h`, `3d`, `7d`, `30d`, este mes, todo, custom).
- Granularidad por hora además de día/semana/mes.
- Buckets y presets mostrados en hora local; datos siguen en UTC.
- Preset sugiere granularidad por defecto (override permitido).
- Guardarraíl: `hora` deshabilitada para rangos largos.

**Non-Goals:**
- Cambiar el esquema de SQLite o el pipeline de ingest.
- Persistir la selección de filtros entre sesiones (puede ser follow-up).
- Zonas horarias configurables manualmente (se usa la del sistema).
- Filtros por proyecto/modelo (fuera de alcance; esto es solo fecha + granularidad).

## Decisions

### D1: Bucketing en hora local vía `strftime(..., 'localtime')`
SQLite ofrece el modificador `'localtime'` que convierte un timestamp UTC a la hora local del host y maneja DST. El bucketing pasa a:

```sql
strftime('<fmt>', timestamp, 'localtime') AS bucket_label
```

con formatos: `hour → '%Y-%m-%d %H:00'`, `day → '%Y-%m-%d'`, `week → '%Y-W%W'`, `month → '%Y-%m'`.

**Alternativa considerada:** convertir en Rust con chrono (como `query_today_by_project`). Rechazada: requiere post-procesar todas las filas y re-agrupar en memoria; `'localtime'` mantiene la agregación en SQL, es menos código y consistente con el índice existente. Se ejecuta en la máquina del usuario, así que `'localtime'` = zona del usuario.

### D2: `since`/`until` con precisión datetime, computados en el frontend
El frontend calcula los límites relativos (`now - 24h`, inicio de mes, etc.) en **hora local**, los convierte a instantes UTC ISO 8601 (`YYYY-MM-DDTHH:MM:SSZ`) y los envía. El backend compara directamente contra `timestamp` (ya UTC):

```sql
WHERE timestamp >= '<since_utc>' AND timestamp <= '<until_utc>'
```

Esto reemplaza la concatenación `'<since>T00:00:00Z'` de día-fijo. Los presets relativos se recomputan en cada fetch, por lo que "últimas 24h" se mantiene fresco con el polling de 30s.

**Alternativa considerada:** enviar tokens relativos (`"last24h"`) y resolverlos en Rust. Rechazada: mete lógica de calendario/zona en el backend; el frontend ya conoce "ahora" local y es el lugar natural para la aritmética de fechas.

### D3: Preset → granularidad sugerida con override
Cada preset define un `defaultBucket`. Al elegir preset se setea esa granularidad, pero el control de granularidad sigue editable. Tabla:

| Preset      | Rango             | defaultBucket |
|-------------|-------------------|---------------|
| 24h         | now-24h → now     | hour          |
| 3 días      | now-3d → now      | hour          |
| 7 días      | now-7d → now      | day           |
| 30 días     | now-30d → now     | day           |
| Este mes    | inicio mes → now  | day           |
| Todo        | earliest → now    | week          |
| Custom      | selección usuario | day           |

### D4: Guardarraíl de cardinalidad para `hora`
Si el rango activo excede ~72h (3 días), la opción `hora` se deshabilita en el control; si estaba activa al ampliar el rango, se degrada a `day`. Evita ~720 barras en un área apilada. El umbral vive en el frontend como constante.

### D5: Estado de filtros en `App.tsx`
Se extiende el estado local (hoy `ChartControlsValue`) para incluir el preset y el rango resuelto (`since`/`until`). No se introduce store global; se mantiene `useState` + `useMemo`, consistente con lo existente. El `SeriesQuery` que arma `useUsageSeries` incluye ahora `since`/`until`.

## Risks / Trade-offs

- **[`'localtime'` depende de la config del host]** → Es el comportamiento deseado (el usuario piensa en su hora local). Si el SO tiene mal la zona, el bucketing la reflejará; aceptable.
- **[Etiquetas de bucket horario más anchas en el eje X]** → Con rangos cortos (≤72h por el guardarraíl) la cantidad de ticks es manejable; se puede ralear ticks en `UsageChart`.
- **[Cambio de formato de `since`/`until` (BREAKING interno)]** → Sin consumidores externos; el único caller es `useUsageSeries`. Se actualizan juntos.
- **[`%W` de semana empieza en lunes/año-cruce]** → Comportamiento preexistente, no se toca en este cambio.
- **[Recomputar presets relativos en cada fetch]** → Costo trivial; asegura frescura sin timers extra.

## Migration Plan

Cambio aditivo en runtime; sin migración de datos (esquema SQLite intacto). Orden de despliegue: backend (`Bucket::Hour` + `'localtime'` + datetime en `WHERE`) y frontend en el mismo release, ya que comparten el contrato de `SeriesQuery`. Rollback = revertir el commit; los datos UTC almacenados no cambian.

## Open Questions

- ¿El preset por defecto al abrir el dashboard debe ser "Todo" (comportamiento actual) o "7 días"? Propuesta: mantener "Todo" para no cambiar la primera impresión; decidir en review.
- ¿Umbral exacto del guardarraíl de `hora` (72h vs 96h)? Propuesta: 72h, ajustable.
