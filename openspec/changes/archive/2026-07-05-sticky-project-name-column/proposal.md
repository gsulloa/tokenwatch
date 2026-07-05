## Why

Cuando la tabla del dashboard tiene muchos buckets, el contenedor hace scroll horizontal y la columna de nombre de serie/proyecto se desplaza fuera de vista, dejando las cifras sin etiqueta identificable. Fijar esa columna mantiene el contexto legible mientras el usuario recorre los períodos.

## What Changes

- La primera columna de la tabla (nombre de serie/proyecto) queda fija ("sticky") durante el scroll horizontal, en encabezado (`thead`), filas de datos (`tbody`) y fila de totales (`tfoot`).
- La columna fija se mantiene por encima de las celdas numéricas al scrollear (capas de `z-index` correctas y fondo opaco para evitar transparencias sobre las columnas que pasan por debajo).
- Se preserva el comportamiento actual del encabezado fijo vertical (`thead` sticky top) y de hover/dimmed de filas.

## Capabilities

### New Capabilities
<!-- Ninguna -->

### Modified Capabilities
- `usage-charts`: la requirement "Tabla de datos con cifras exactas" añade que la columna de nombre de serie/proyecto permanece fija durante el scroll horizontal.

## Impact

- `packages/app/src/styles/global.css` — reglas de `.usage-table__series-cell` (y su interacción con `.usage-table thead`/`tfoot`) para `position: sticky; left: 0`.
- `packages/app/src/features/usage/UsageTable.tsx` — sin cambios de markup previstos (la clase existente ya se aplica en las tres ubicaciones); solo se ajusta si hace falta apoyar los estilos.
- Sin cambios de datos, APIs ni dependencias.
