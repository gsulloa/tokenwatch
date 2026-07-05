## Context

`query_series_inner()` (`packages/app/src-tauri/src/usage/mod.rs:263`) agrega eventos con un `GROUP BY strftime(...)` en SQLite y deriva la lista de buckets (`buckets: Vec<String>`) **de las filas devueltas** por ese query (líneas 334-342). Como `GROUP BY` solo produce filas para buckets con eventos, los buckets vacíos nunca entran en la lista. El relleno con 0 posterior (líneas 358-373) alinea cada serie contra `buckets`, pero `buckets` ya viene incompleto, así que solo rellena huecos *entre* buckets existentes, no el rango completo.

Las etiquetas de bucket son strings formateados por SQLite con `strftime(fmt, timestamp, 'localtime')` (`%Y-%m-%d %H:00` hora, `%Y-%m-%d` día, `%Y-W%W` semana, `%Y-%m` mes). El frontend (`UsageChart.tsx`, tabla) consume `buckets` + `series` sin transformarlos, así que cualquier bucket que agregue el backend aparece automáticamente en el eje X y en la tabla.

## Goals / Non-Goals

**Goals:**
- Devolver **todos** los buckets del rango efectivo (según granularidad), con 0 en los que no tienen eventos.
- Mantener las etiquetas de bucket idénticas a las que ya produce el query de datos (mismo formato, misma hora local, mismo comportamiento DST).
- No cambiar el contrato `SeriesResponse` ni el frontend.

**Non-Goals:**
- Reimplementar el cálculo de buckets/zonas horarias en Rust con `chrono`/`chrono-tz`.
- Cambiar el estado vacío: si no hay **ningún** evento en el rango, se sigue devolviendo respuesta vacía (el dashboard muestra su "Estado vacío"); no se fabrica un gráfico plano en 0.
- Cambiar presets, granularidad o guardarraíles del frontend.

## Decisions

### D1: Enumerar los buckets en SQLite, no en Rust
Se genera la secuencia completa de etiquetas de bucket con un **CTE recursivo** en SQLite que parte del inicio del rango (redondeado hacia abajo al borde del bucket, en hora local) y avanza con el paso propio de la granularidad, formateando cada paso con el **mismo `strftime`** que el query de datos.

- **Por qué:** SQLite es la única fuente de verdad del formato de etiqueta y de la semántica `localtime` (incluida la numeración de semana `%W` y los saltos de DST). Generar las etiquetas con el mismo motor garantiza que las claves de enumeración hagan match exacto con las claves del `value_map`; cualquier reimplementación en Rust arriesga desalineación (p.ej. `%W` vs. `chrono`, o DST).
- **Alternativa considerada:** enumerar con `chrono` + `chrono-tz` en Rust. Rechazada por el riesgo de divergencia de formato/zona horaria y por introducir dependencia y lógica de DST duplicada.

Paso por granularidad (anclado a un datetime local, no al string de etiqueta):
- Hora: arranque `strftime('%Y-%m-%d %H:00:00', lo, 'localtime')`, paso `datetime(x, '+1 hour')`.
- Día: arranque `date(lo, 'localtime')`, paso `date(x, '+1 day')`.
- Semana: arranque al inicio de semana local, paso `+7 days`.
- Mes: arranque `date(lo, 'localtime', 'start of month')`, paso `date(x, '+1 month')`.
La etiqueta de cada paso se formatea con el `strftime_fmt` correspondiente para reproducir exactamente las claves del query de datos.

### D2: Bounds del rango — `since`/`until` cuando existan; si no, min/max de datos
El CTE se acota entre `lo` y `hi`:
- Si `params.since`/`params.until` vienen definidos, se usan como bounds (convertidos a hora local dentro del CTE).
- Si faltan, se derivan de `MIN(timestamp)`/`MAX(timestamp)` de los eventos que cumplen el `where_clause`.
El bucket que contiene `hi` es **inclusivo** (la condición de parada del CTE compara contra el inicio del bucket de `hi`, no contra `hi` exacto), para no perder la hora/día final.

- **Por qué:** replica el comportamiento actual (rango implícito = datos) para el preset "todo", pero ahora rellena los intermedios; y respeta el rango exacto de los presets con `since`/`until`.

### D3: Cambio mínimo y localizado en `query_series_inner`
Se introduce un helper que devuelve `Vec<String>` con las etiquetas completas ordenadas (ejecutando el CTE) y se **reemplaza** el bloque de derivación de `buckets` (líneas 334-342) por su resultado. El resto —`series_names` desde las filas reales, `value_map`, y el ensamblado con `unwrap_or(0.0)` (líneas 358-373)— queda igual: ya rellena en 0 contra la lista de buckets que reciba.

- **Por qué:** el 0-fill existente ya es correcto; solo hay que darle la lista de buckets completa. Minimiza superficie de cambio y riesgo.
- `series_names` se sigue tomando de los datos: una serie sin ningún evento en el rango no aparece (correcto; no inventamos series).

### D4: Salvaguarda de cardinalidad
La enumeración hereda el guardarraíl del frontend (hora deshabilitada para rangos > ~72h), por lo que el máximo de buckets-hora es acotado (~72). Para día/semana/mes la cardinalidad es naturalmente baja. Aun así, el helper acota defensivamente el número de iteraciones del CTE (p.ej. límite duro razonable) para evitar un CTE runaway ante bounds corruptos.

## Risks / Trade-offs

- **[Desalineación de etiquetas entre enumeración y datos]** → Mitigación: ambos usan el mismo `strftime_fmt` y `'localtime'` en el mismo motor SQLite; se agregan tests que verifican match exacto de claves (hora con huecos, cruce de medianoche).
- **[Semana `%W` en bordes de año]** → Mitigación: al formatear con el mismo `strftime` que los datos, el número de semana coincide por construcción; test específico de rango que cruza fin de año en granularidad semana.
- **[DST: hora local con salto/retroceso]** → Mitigación: el paso `+1 hour` sobre datetime local en SQLite es consistente con cómo se agrupan los datos; se documenta como comportamiento esperado (no se intenta "corregir" la hora repetida/faltante más allá de lo que hace SQLite).
- **[Más puntos en el eje X → densidad de ticks]** → Mitigación: `computeTickInterval()` en `UsageChart.tsx` ya ralea ticks por hora; sin cambios necesarios, se verifica visualmente.
- **[Rango sin datos]** → No se rellena (Non-Goal): se mantiene el estado vacío para no mostrar un gráfico plano engañoso.
