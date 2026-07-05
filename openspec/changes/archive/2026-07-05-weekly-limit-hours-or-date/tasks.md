## 1. Formato del label de reset

- [x] 1.1 En `packages/app/src/features/limits/LimitGauge.tsx`, añadir una constante para el umbral de 12h (`12 * 60 * 60 * 1000`).
- [x] 1.2 Añadir la función `formatResetDate(resetsAt)` que use `toLocaleDateString("es", { weekday: "short", day: "numeric", month: "short" })` y devuelva `"cierra el <fecha>"`, envuelta en try/catch → `""`.
- [x] 1.3 En `formatResetClock`, pasar locale explícito `"es"` a `toLocaleTimeString` (en vez de `[]`).

## 2. Lógica de selección por umbral

- [x] 2.1 En el cuerpo de `LimitGauge`, calcular `diffMs = new Date(resetsAt).getTime() - Date.now()` (con guarda para `resetsAt` vacío/ inválido).
- [x] 2.2 Construir `resetLabel` así: si `diffMs >= 12h` usar solo `formatResetDate(resetsAt)`; si `diffMs < 12h` (incluye ≤ 0) mantener el comportamiento actual `[formatTimeUntilReset, formatResetClock].filter(Boolean).join(" · ")`.
- [x] 2.3 Verificar que la ventana de sesión (5h) mantiene "resetea en Xh Ym · HH:MM TZ" sin cambios, y que los medidores compactos por modelo aplican el mismo umbral.

## 3. Verificación

- [x] 3.1 `pnpm typecheck && pnpm lint && pnpm test:run` pasan.
- [ ] 3.2 Comprobar visualmente en el popover: semana lejana muestra "cierra el <fecha>", semana a < 12h muestra "resetea en Xh Ym", sesión sin cambios.
