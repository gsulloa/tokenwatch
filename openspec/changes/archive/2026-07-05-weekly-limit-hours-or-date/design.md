## Context

Hoy `LimitGauge.tsx` es el único componente que renderiza los medidores de límites, y se usa para las tres clases de ventana (sesión 5h, semana total, y semanales por modelo). Construye el label de reset combinando dos helpers:

- `formatTimeUntilReset(resetsAt)` → "resetea en Xh Ym" (o "resetea ahora").
- `formatResetClock(resetsAt)` → "20:00 GMT-4".

Y los une: `const resetLabel = [timeLabel, clockLabel].filter(Boolean).join(" · ")`.

El requisito es que, para la ventana semanal, cuando el reset está lejos (≥ 12h) se muestre una **fecha de cierre** en vez de las horas restantes. Como la ventana de sesión nunca llega a 12h, aplicar el umbral de forma **universal** en el helper deja intacto su comportamiento y evita tener que pasar un flag de tipo de ventana desde `LimitsSection`.

Constraints: app solo-español, strings hardcodeados, sin i18n. Sin cambios de backend — `resetsAt` (ISO 8601) ya está disponible en el componente.

## Goals / Non-Goals

**Goals:**
- Medidor semanal: mostrar horas restantes solo cuando falten < 12h; en caso contrario, mostrar la fecha de cierre.
- Cambio contenido en `LimitGauge.tsx`, sin tocar backend, tipos ni `LimitsSection`.
- Mantener el comportamiento de la ventana de sesión sin cambios visibles.

**Non-Goals:**
- No se cambia el poll de límites, la API ni el modelo de datos.
- No se introduce i18n ni configuración del umbral (12h es constante).
- No se cambia el formato del clock/hora para la ventana de sesión.

## Decisions

**Decisión 1: Aplicar el umbral de 12h dentro de `LimitGauge`, de forma universal.**
El branch por umbral vive en la lógica de formato del label, no en `LimitsSection`. Rationale: la ventana de sesión (5h) nunca supera 12h, así que el umbral no la afecta; evita añadir un prop `variant`/`isWeekly` y mantiene el componente auto-contenido.
- Alternativa considerada: pasar un prop desde `LimitsSection` para aplicar el branch solo a semana/por-modelo. Descartada por ser más código sin cambio de comportamiento observable.

**Decisión 2: Umbral basado en el mismo `diffMs` que ya se calcula.**
Se reutiliza `target - now`. Si `diffMs < 12 * 3_600_000` → rama de horas restantes (comportamiento actual). Si `diffMs >= 12h` → rama de fecha de cierre. `diffMs <= 0` sigue devolviendo "resetea ahora".

**Decisión 3: Formato de la fecha de cierre.**
Nueva función `formatResetDate(resetsAt)` usando `toLocaleDateString` con locale explícito español y `{ weekday: "short", day: "numeric", month: "short" }`, prefijada con "cierra el ". Ej.: `"cierra el sáb 12 jul"`. Rationale: día de la semana + día + mes es legible para un horizonte de hasta ~7 días sin ser verboso; no se incluye el año porque las ventanas semanales nunca cruzan un año de distancia.
- Cuando se muestra la fecha, se **omite el clock** ("20:00 GMT-4") para no saturar; el clock solo aporta valor cuando el reset es inminente. Así, la rama ≥12h muestra solo la fecha, y la rama <12h conserva "resetea en Xh Ym · 20:00 GMT-4".
- Locale: pasar `"es"` explícito a `toLocaleDateString`/`toLocaleTimeString` para no depender del locale del sistema (la app es solo-español).

**Decisión 4: Mantener tolerancia a errores.** Las funciones de formato siguen envueltas en try/catch devolviendo `""`, y `resetLabel` sigue filtrando vacíos, de modo que un `resetsAt` inválido no rompe el render.

## Risks / Trade-offs

- **Redondeo cerca del umbral (11h59m ↔ 12h00m)** → el label puede cambiar entre horas y fecha en re-renders muy próximos al borde. Aceptable: es un cambio cosmético y poco frecuente.
- **Formato de fecha dependiente de locale** → mitigado pasando `"es"` explícito en lugar del locale del sistema.
- **Semanales por modelo también cambian de formato** → es consistente y deseado (mismo tipo de ventana semanal); documentado en el spec.
