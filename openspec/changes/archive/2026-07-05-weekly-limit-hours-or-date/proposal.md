## Why

El medidor de la ventana **semanal** en el popover siempre muestra "resetea en Xh Ym", pero cuando faltan varios días para el reset esa cifra en horas (p.ej. "resetea en 137h") es difícil de interpretar. Una fecha de cierre es más legible cuando el reset está lejos, mientras que las horas restantes siguen siendo útiles cuando el cierre es inminente.

## What Changes

- El medidor de la ventana **semanal** SHALL mostrar el **tiempo restante** ("resetea en Xh Ym") solo cuando falten **menos de 12 horas** para el `resets_at`.
- Cuando falten **12 horas o más**, el medidor semanal SHALL mostrar la **fecha de cierre** del período (p.ej. "cierra el sáb 12 jul") en lugar de las horas restantes.
- La ventana de **sesión (5h)** conserva su comportamiento actual (siempre horas restantes), lo que ocurre de forma natural porque su reset nunca supera las 12 horas.

## Capabilities

### New Capabilities
<!-- Ninguna. -->

### Modified Capabilities
- `menubar-popover`: cambia cómo el medidor semanal presenta el reset — horas restantes solo bajo el umbral de 12 horas, y fecha de cierre en caso contrario.

## Impact

- `packages/app/src/features/limits/LimitGauge.tsx`: lógica de formato del label de reset (nueva ramificación por umbral de 12h y formateo de fecha).
- Sin cambios en el backend Rust ni en la API de límites; se usa el `resets_at` ya disponible.
- Sin cambios de dependencias. La app es solo-español con strings hardcodeados.
