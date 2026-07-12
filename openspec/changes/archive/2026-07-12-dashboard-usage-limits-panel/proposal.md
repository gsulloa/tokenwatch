## Why

Hoy la información "en vivo" (límites 5h / semana / por modelo, presupuestos por
grupo y consumo de hoy por proyecto) sólo existe en el popover de la barra de
menú. Cuando el usuario abre el dashboard para analizar el histórico, pierde de
vista justo la señal más importante del producto: *cuánto combustible queda y a
qué velocidad se quema*. El dashboard es la ventana grande y persistente donde
el usuario ya está trabajando; debería mostrar el mismo estado de presión que el
popover, para verlo en ambos lados sin tener que volver a la barra de menú.

## What Changes

- El **dashboard** (ventana principal) SHALL mostrar las mismas lecturas en vivo
  que el popover: medidores de límites (sesión 5h, semana, semanales por modelo),
  presupuestos por grupo (cuando existan grupos definidos) y consumo de hoy por
  proyecto.
- Se extrae la composición compartida de esas tres secciones a un componente
  reutilizable (p. ej. `LiveStatusPanel`) que encapsula el cableado de los hooks
  `useLimits`, `useGroupBudgets` y `useTodayByProject` y su render con separadores.
- El **popover** SHALL seguir mostrando exactamente lo mismo que hoy, pero
  renderizando ese componente compartido en lugar de su composición inline
  (para garantizar paridad y evitar divergencia futura).
- Ambas superficies SHALL reaccionar a los mismos eventos del backend
  (`limits-updated`, etc.), de modo que los medidores se mantengan sincronizados
  sin importar qué ventana esté abierta.
- No hay cambios de backend: se reutilizan los comandos y eventos Tauri
  existentes (`query_limits`, `query_today_by_project`, presupuestos por grupo).

## Capabilities

### New Capabilities
- `dashboard-status-panel`: el dashboard muestra las lecturas en vivo (límites,
  presupuestos por grupo y consumo de hoy por proyecto) además del análisis
  histórico, reutilizando la misma presentación que el popover.

### Modified Capabilities
- (Ninguna.) El contenido y comportamiento visible del popover no cambia. La
  extracción a un componente compartido es un detalle de implementación
  (ver `design.md`), no un cambio a nivel de requisito de `menubar-popover`.

## Impact

- **Código afectado (frontend)**: `packages/app/src/app/App.tsx` (dashboard),
  `packages/app/src/app/Popover.tsx` (refactor a componente compartido), y un
  nuevo componente compartido bajo `packages/app/src/features/` (p. ej.
  `live-status/LiveStatusPanel.tsx`).
- **Reutiliza sin cambios**: `LimitsSection`, `GroupBudgetsSection`,
  `TodayByProjectList` y los hooks `useLimits`, `useGroupBudgets`,
  `useTodayByProject`.
- **Backend**: sin cambios. La ventana del dashboard pasará a suscribirse a
  `limits-updated` y a invocar `query_limits` / `query_today_by_project` al
  montarse (una consulta adicional por apertura de ventana, tolerable).
- **Diseño**: nueva ubicación en el dashboard para el panel de estado en vivo;
  debe respetar `DESIGN.md` (denso, instrumento, sin cards, rieles con ticks).
- **Sin impacto** en infra, releases ni tests de backend.
