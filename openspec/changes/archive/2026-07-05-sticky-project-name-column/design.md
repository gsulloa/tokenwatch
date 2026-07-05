## Context

La tabla del dashboard (`UsageTable.tsx`) vive en un wrapper `.usage-table-wrapper` con `overflow-x: auto`. El encabezado (`thead`) ya es sticky vertical (`position: sticky; top: 0; z-index: 1`). La primera columna, `.usage-table__series-cell`, aparece en `thead`, `tbody` y `tfoot` con la misma clase, pero hoy scrollea con el resto de la tabla.

Es un cambio CSS puro; el markup ya aplica la misma clase en las tres ubicaciones, así que una sola regla las cubre.

## Goals / Non-Goals

**Goals:**
- La columna de nombre queda fija a la izquierda durante scroll horizontal, en `thead`, `tbody` y `tfoot`.
- La celda de encabezado de esa columna queda fija en ambos ejes (esquina superior-izquierda).
- Fondo opaco en la columna fija para que las celdas numéricas pasen por debajo sin verse a través.

**Non-Goals:**
- No se fijan otras columnas (p.ej. la de Total a la derecha).
- No se cambia el ancho, orden ni el contenido de la tabla.
- No se toca la lógica de datos ni el markup React (salvo apoyo mínimo si algún estilo lo requiere).

## Decisions

- **`position: sticky; left: 0` sobre `.usage-table__series-cell`** en lugar de un layout de dos tablas o JS de scroll-sync. Sticky es nativo, sin JS, y ya se usa para el `thead`. Alternativa descartada: dividir en dos tablas sincronizadas (más complejo, propenso a desalineación de alturas de fila).
- **Capas de `z-index`**: celdas de cuerpo fijas por encima de las numéricas (`z-index: 2`); la celda de encabezado de la columna (intersección sticky-top + sticky-left) por encima de todo (`z-index: 3`). El `thead` normal se mantiene en su capa actual.
- **Fondo opaco**: `.usage-table__series-cell` en `tbody` necesita un `background` sólido (no transparente) igual al fondo de la fila, incluyendo el estado hover/dimmed, para que al scrollear no se transparente la columna numérica que pasa detrás. Las celdas de `thead`/`tfoot` ya tienen fondo `--surface-2`.

## Risks / Trade-offs

- [Fondo hover no coincide en la celda fija] → aplicar el mismo `background` en `.usage-table__row:hover .usage-table__series-cell` / `--hovered`, y respetar el `opacity` de `--dimmed` (se hereda de la fila, no requiere override).
- [`overflow: hidden` + `text-overflow: ellipsis` ya presentes en la celda] → sticky es compatible; conservar `max-width: 220px` y truncado.
- [Borde derecho de separación] → opcional añadir `border-right` sutil a la columna fija para marcar el límite al scrollear; bajo riesgo visual.
