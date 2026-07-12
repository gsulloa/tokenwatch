## Context

TokenWatch tiene dos superficies (ver `DESIGN.md`):

1. **Popover** (`packages/app/src/app/Popover.tsx`, ventana Tauri `popover`) —
   compacto y denso. Hoy muestra, en este orden: `LimitsSection` (sesión 5h,
   semana, semanales por modelo), `GroupBudgetsSection` (condicional a que haya
   grupos definidos), `TodayByProjectList`, `UpdateBanner` y una fila de comandos
   (silenciar alertas, modo de badge, "Acerca de", "Abrir dashboard").
2. **Dashboard** (`packages/app/src/app/App.tsx`, ventana `main`) — ventana
   grande de análisis histórico. Hoy muestra KPIs, `ChartControls`, `UsageChart`,
   `UsageTable` y `GroupsEditor`.

`main.tsx` decide qué árbol montar según `getCurrentWindow().label`
(`popover` → `<Popover/>`, resto → `<App/>`).

Puntos clave del estado actual que habilitan este cambio:

- Las tres secciones en vivo son **presentacionales puras**: reciben
  `snapshot`/`data` + `loading` y no hacen fetch por sí mismas
  (`LimitsSection`, `GroupBudgetsSection`, `TodayByProjectList`).
- Los tres hooks son **autocontenidos** y reutilizables entre ventanas:
  `useLimits` hace fetch en mount y se suscribe al evento Tauri `limits-updated`;
  `useGroupBudgets` y `useTodayByProject` siguen el mismo patrón (funcionan hoy
  en el popover). Fuera de Tauri (jsdom/tests) degradan a no-op.
- El backend emite eventos globales (`limits-updated`) independientes de qué
  ventana esté abierta, y expone `query_limits` / `query_today_by_project` y los
  comandos de presupuestos. **No requiere cambios.**

La restricción de diseño es que el dashboard debe respetar `DESIGN.md`: denso,
"instrumento", sin KPI cards decorativas, rieles con ticks 70/85/100, morado sólo
en chrome. `DESIGN.md` ya nombra "editor de grupos como inspector lateral", lo que
avala un rail lateral en el dashboard.

## Goals / Non-Goals

**Goals:**
- Mostrar en el dashboard las mismas lecturas en vivo que el popover: límites,
  presupuestos por grupo y consumo de hoy por proyecto.
- Garantizar **paridad** entre ambas superficies: una sola fuente de composición,
  de modo que agregar/quitar una sección en el futuro se refleje en ambos lados.
- Mantener el popover visualmente idéntico a hoy.
- Cero cambios de backend.

**Non-Goals:**
- No se rediseñan las secciones en vivo ni sus medidores.
- No se añade el gráfico histórico ni la tabla al popover (el flujo es
  unidireccional: la info del popover entra al dashboard, no al revés).
- No se cambia el modelo de datos, comandos ni eventos Tauri.
- No se unifican las ventanas ni el enrutado de `main.tsx`.

## Decisions

### Decisión 1: Extraer un componente compartido `LiveStatusPanel`

Crear `packages/app/src/features/live-status/LiveStatusPanel.tsx` que:
- Internamente cablea `useLimits`, `useGroupBudgets` y `useTodayByProject`.
- Renderiza, con los mismos separadores hairline de hoy y en el mismo orden:
  `LimitsSection` → (condicional) `GroupBudgetsSection` → `TodayByProjectList`.
- Replica la lógica `hasDefinedGroups` actual del popover para decidir si muestra
  la sección de presupuestos.
- Expone la primitiva de refresco (`refreshIfStale` de límites + `refresh` de
  hoy) para que cada superficie pueda dispararla en sus propios eventos
  (popover: `popover-shown`; dashboard: botón "Actualizar").

Se puede exponer el refresco vía un `ref` imperativo (`useImperativeHandle`) o
vía un prop callback `onReady(controls)`. **Elegido:** `forwardRef` +
`useImperativeHandle` exponiendo `{ refresh(), refreshIfStale() }`, porque el
popover ya orquesta el refresco desde su efecto de `popover-shown` y necesita una
referencia estable sin re-render.

**Alternativa considerada:** duplicar la composición en `App.tsx`. Rechazada:
divergiría del popover con el tiempo (la razón misma del bug de "sólo en un
lado"). Un único componente compartido es la garantía de paridad.

**Alternativa considerada:** subir los hooks a un contexto compartido montado en
`main.tsx` para ambas ventanas. Rechazada: las ventanas son procesos/webviews
separados (no comparten estado JS), así que un contexto no ahorra fetches entre
ventanas; sólo agrega complejidad. Cada ventana instancia sus propios hooks.

### Decisión 2: Ubicación en el dashboard — rail lateral derecho

Reestructurar `dashboard__content` a dos columnas: columna principal (KPIs,
controles, gráfico, tabla) + **rail lateral** con `<LiveStatusPanel/>` fijo
arriba (sticky). En anchos angostos, colapsa a una sola columna apilando el panel
en vivo **encima** del análisis histórico (la presión va primero).

Esto respeta la densidad y el lenguaje "inspector lateral" de `DESIGN.md`, y deja
el gráfico como panel de primera clase en la columna principal. El
`GroupsEditor` (edición de grupos) permanece donde está; `GroupBudgetsSection`
(consumo) vive en el rail — son piezas distintas (editar vs. medir).

**Alternativa considerada:** banda superior horizontal de ancho completo debajo
del topbar. Rechazada: `TodayByProjectList` es una lista vertical rankeada y los
medidores por modelo crecen en vertical; forzarlos a una banda horizontal rompe
la densidad. Un rail vertical los aloja naturalmente.

### Decisión 3: El popover consume el mismo componente

`Popover.tsx` reemplaza su composición inline de las tres secciones por
`<LiveStatusPanel ref={...}/>`, conservando su chrome propio (`UpdateBanner`,
toggle de silenciar, selector de badge, "Acerca de", "Abrir dashboard") y su
efecto `popover-shown` que ahora llama al refresco del panel vía el ref.

### Decisión 4: Refresco y sincronización en el dashboard

`useLimits` en el dashboard hace fetch al montar y se actualiza vía
`limits-updated` (emitido por el poll de backend cada 5 min). El botón
"Actualizar" existente del dashboard, además de refrescar la serie histórica,
SHALL invocar `LiveStatusPanel.refresh()` para forzar límites + hoy. No se añade
un poll nuevo; se reusa el evento global existente.

## Risks / Trade-offs

- **[Fetch adicional por ventana]** Abrir el dashboard dispara un `query_limits`
  y un `query_today_by_project` extra (cada ventana tiene sus hooks). → Mitigación:
  `useLimits` ya throttlea con `refreshIfStale` (10 s); el costo es una consulta
  al abrir, aceptable. El poll periódico sigue siendo único en backend.
- **[Divergencia de layout popover vs. dashboard]** El componente compartido debe
  ser agnóstico del ancho. → Mitigación: `LiveStatusPanel` no fija ancho; el
  contenedor (popover ~360px / rail del dashboard) define el ancho. Las secciones
  ya usan tokens y `flex-direction: column`, así que fluyen a cualquier ancho.
- **[Reflow del dashboard]** Pasar a dos columnas cambia el layout actual. →
  Mitigación: mantener `max-width` 1120–1200px de `DESIGN.md`; degradar a una
  columna bajo un breakpoint; QA visual contra `DESIGN.md` (rieles, sin cards,
  hairlines).
- **[Tests]** Fuera de Tauri los hooks degradan a no-op y las secciones muestran
  estados vacíos; `App.test.tsx` debe seguir pasando con el panel presente. →
  Mitigación: el panel renderiza estados "Sin datos" sin lanzar en jsdom.

## Migration Plan

Cambio puramente aditivo en frontend, sin migración de datos ni breaking changes.
Rollback = revertir el commit (el popover vuelve a su composición inline). No hay
estado persistido nuevo.

## Open Questions

- ¿El rail lateral debe ser sticky en scroll o fijo dentro del flujo? (recomendado
  sticky top). Resolver en implementación/QA de diseño.
- ¿El botón "Actualizar" del dashboard debe refrescar también límites+hoy, o basta
  con el evento global? (recomendado: sí refrescar, por consistencia con el gesto
  del usuario).
