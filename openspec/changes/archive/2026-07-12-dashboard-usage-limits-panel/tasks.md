## 1. Componente compartido `LiveStatusPanel`

- [x] 1.1 Crear `packages/app/src/features/live-status/LiveStatusPanel.tsx` que cablee internamente `useLimits`, `useGroupBudgets` y `useTodayByProject`.
- [x] 1.2 Renderizar dentro del panel, en este orden y con los mismos separadores hairline del popover actual: `LimitsSection` → (condicional) `GroupBudgetsSection` → `TodayByProjectList`.
- [x] 1.3 Portar la lógica `hasDefinedGroups` (del `Popover.tsx` actual, líneas 124-128) al panel para decidir si se muestra la sección de presupuestos y sus separadores.
- [x] 1.4 Exponer un handle imperativo vía `forwardRef` + `useImperativeHandle` con `{ refresh(), refreshIfStale() }` que dispare el refresco de límites y de hoy.
- [x] 1.5 Asegurar que el panel no fija ancho propio (fluye al ancho del contenedor) y que degrada sin lanzar en entornos no-Tauri (jsdom/tests).

## 2. Refactor del popover a la fuente compartida

- [x] 2.1 En `packages/app/src/app/Popover.tsx`, reemplazar la composición inline de `LimitsSection` + `GroupBudgetsSection` + `TodayByProjectList` (y sus separadores/hooks) por `<LiveStatusPanel ref={...}/>`.
- [x] 2.2 Conservar el chrome propio del popover fuera del panel: `UpdateBanner`, toggle de silenciar, selector de badge, "Acerca de", "Abrir dashboard".
- [x] 2.3 Ajustar el efecto `popover-shown` para llamar al refresco del panel a través del ref (en vez de a los hooks locales, ya eliminados).
- [x] 2.4 Verificar visualmente que el popover queda idéntico a antes (mismas secciones, orden, separadores, estados vacíos).

## 3. Integración en el dashboard

- [x] 3.1 En `packages/app/src/app/App.tsx`, montar `<LiveStatusPanel ref={...}/>` en el dashboard.
- [x] 3.2 Reestructurar `dashboard__content` a layout de dos columnas: columna principal (KPIs, controles, gráfico, tabla, editor de grupos) + rail lateral con el panel en vivo (sticky top).
- [x] 3.3 Añadir estilos en `packages/app/src/styles/global.css` para el rail y el layout de dos columnas, respetando `max-width` 1120–1200px.
- [x] 3.4 Colapsar a una sola columna bajo un breakpoint angosto, apilando el panel en vivo **encima** del análisis histórico.
- [x] 3.5 Hacer que el botón "Actualizar" del dashboard, además de `refresh` de la serie histórica, invoque `LiveStatusPanel.refresh()`.

## 4. Diseño y QA

- [x] 4.1 Revisar el resultado contra `DESIGN.md`: sin KPI cards decorativas nuevas, rieles con ticks 70/85/100, hairlines en vez de sombras, morado sólo en chrome, densidad compacta.
- [x] 4.2 Verificar sincronización en vivo del dashboard: emitir/observar `limits-updated` y `usage-updated` y confirmar que medidores y hoy-por-proyecto se actualizan sin recargar la ventana.
- [x] 4.3 Verificar estados de error/no-disponible en el dashboard (token expirado, sin login, keychain denegado) muestran el mismo mensaje que el popover.

## 5. Tests y verificación final

- [x] 5.1 Actualizar/añadir tests de render: el panel muestra estados vacíos sin lanzar en jsdom; `App.test.tsx` sigue pasando con el panel presente.
- [x] 5.2 Añadir un test que confirme la paridad de composición (popover y dashboard renderizan las mismas secciones del panel compartido).
- [x] 5.3 Ejecutar `pnpm typecheck && pnpm lint && pnpm test:run` y dejarlos en verde.
