## Why

Desde fines de octubre de 2026 (macOS 27.0.1, app v0.2.6) aparecen dos regresiones:

1. **El título del tray muestra solo `–`.** `/api/oauth/usage` ahora devuelve un `limits[]` no vacío sin fila `kind: "session"`, y el `five_hour` top-level pasó a ser opcional: solo existe mientras la ventana de 5h está activa y su `resets_at` no ha pasado. El parser solo usa los campos top-level cuando `limits[]` está vacío, así que la sesión queda siempre en `None`. Con la etiqueta en modo `session`, el título cae a `–`. Además, un `five_hour` con campos `null` o un `limits: null` harían fallar el parseo completo.
2. **Hacer click en el icono del tray ya no muestra el popover.** Solo aparece después de abrir el dashboard al menos una vez. El código no cambió desde julio, así que es casi seguro que la causa es la interacción entre macOS 27 y el NSPanel no-activante de una app `Accessory` que nunca se ha activado: `hidesOnDeactivate`, una máscara de estilo reemplazada o un `resign key` espurio. También podría influir el estado de ventanas restaurado.

## What Changes

**Parseo de límites:**
- Cada ventana se deriva por separado: `limits[]` tiene prioridad, y `five_hour` / `seven_day` / `seven_day_*` cubren la ventana que falte.
- El parseo tolera `limits: null`, filas de `limits[]` mal formadas (se descartan individualmente) y campos top-level en `null`.
- Una sesión cuyo `resets_at` ya pasó se descarta.

**Etiqueta de la barra de menú:**
- En modo `session`, si no hay sesión activa, se muestra el % semanal con el prefijo `W ` (p.ej. `W 56%`) en vez de `–`.
- `–` queda reservado para límites no disponibles o para cuando no exista ninguna ventana aplicable.

**Popover, contenido:**
- Si hay semana pero no sesión, el medidor de sesión muestra "Sin sesión activa" en vez de desaparecer.

**Popover, apertura (macOS):**
- Se añaden trazas de diagnóstico.
- El bit no-activante se combina con OR en la máscara de estilo existente en vez de reemplazarla.
- Se fija `hidesOnDeactivate = NO`.
- Nivel y máscara se tipan con `objc2-app-kit`, lo que corrige el ABI `i32`.
- El show se difiere a la siguiente vuelta del run loop y se verifica después.
- Se ignora un `resign key` dentro de un periodo de gracia tras mostrar.
- Un click en el icono con el popover abierto lo cierra sin reabrirlo.

**Persistencia de ventanas (`tauri-plugin-window-state`):**
- Se excluye el `popover`.
- En `main` no se restauran ni `FULLSCREEN` ni `VISIBLE`.

## Capabilities

### New Capabilities

(ninguna)

### Modified Capabilities

- `usage-limits`: derivación por ventana con respaldo top-level aunque `limits[]` exista; parseo tolerante a `null` y a filas mal formadas; descarte de sesión vencida.
- `menubar-badge`: respaldo al % semanal (con prefijo `W `) en modo `session` cuando no hay sesión activa; redefinición de cuándo se muestra `–`.
- `menubar-popover`: apertura desde el primer click sin haber activado la app, periodo de gracia ante pérdida de foco, cierre al volver a hacer click en el icono, exclusiones de persistencia de ventanas, y estado "Sin sesión activa" en el medidor de sesión.

## Impact

- `packages/app/src-tauri/src/limits/mod.rs`: structs de respuesta, `parse_api_response` y tests nuevos de parseo con JSON.
- `packages/app/src-tauri/src/menubar.rs`: `badge_text` y tests.
- `packages/app/src-tauri/src/lib.rs`: conversión a NSPanel, handler del tray, delegado `resign key` y configuración de `tauri-plugin-window-state`.
- Nuevo `packages/app/src-tauri/src/popover.rs`: lógica de tiempos (gracia/debounce), testeable, y diagnóstico del panel.
- `packages/app/src/features/limits/LimitsSection.tsx`, más un test nuevo `LimitsSection.test.tsx`.
- Sin dependencias nuevas (`objc2-app-kit` ya está). Actualizar `tauri-nspanel` queda fuera de alcance.
- Usuarios con `.window-state.json` heredado: el dashboard deja de restaurarse en pantalla completa o visible al arrancar.
