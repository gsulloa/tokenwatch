## 1. Columna fija (CSS)

- [x] 1.1 En `packages/app/src/styles/global.css`, añadir a `.usage-table__series-cell`: `position: sticky; left: 0; z-index: 2;` y un `background` opaco (usar `--surface` para el fondo base de filas).
- [x] 1.2 Asegurar que la celda de encabezado de esa columna (intersección `thead` + primera columna) quede por encima con `z-index: 3` y fondo `--surface-2`, sin romper el sticky vertical existente del `thead`.
- [x] 1.3 Mantener el fondo correcto de la columna fija en estados de fila: `.usage-table__row:hover .usage-table__series-cell` y `.usage-table__row--hovered .usage-table__series-cell` con fondo `--surface-2`; verificar que `--dimmed` (opacity) se herede sin override.
- [x] 1.4 Aplicar/confirmar fondo opaco en la celda de nombre del `tfoot` (fila de totales) para que no se transparente al scrollear.
- [x] 1.5 (Opcional) Añadir `border-right: 1px solid var(--border)` a la columna fija para marcar el límite durante el scroll horizontal.

## 2. Verificación

- [ ] 2.1 Con datos que generen muchos buckets, hacer scroll horizontal y confirmar que la columna de nombre permanece fija y legible, sin transparencias ni solapamientos. _(QA visual pendiente en la app)_
- [ ] 2.2 Confirmar que el encabezado sigue fijo verticalmente y que hover/dimmed de filas se ven correctos en la columna fija. _(QA visual pendiente en la app)_
- [x] 2.3 Ejecutar `pnpm typecheck && pnpm lint && pnpm test:run` en `packages/app`.
