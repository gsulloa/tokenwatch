## Why

En macOS la barra de menú se llena de iconos y, cuando hay muchos, el icono de
TokenWatch queda oculto (desborde o detrás del notch), por lo que el usuario
pierde de vista su consumo. macOS **no** permite que una app fije la prioridad ni
el orden de su ítem frente a los de otras apps —ese orden lo controla el usuario
(⌘-arrastrar) y el sistema—, así que no podemos "forzar" que el icono no se
esconda. Lo que sí podemos hacer, y es lo que realmente resuelve el problema, es
llevar la información **al propio ítem de la barra de menú**: mostrar el
porcentaje de uso como texto junto al icono, de modo que el dato esté siempre a
la vista de un vistazo (sin clic) y el ítem sea más ancho, más reconocible y
"valga la pena" mantenerlo fijado a la izquierda.

## What Changes

- Añadir una **etiqueta de texto en la barra de menú** junto al icono del tray
  (vía `TrayIcon::set_title`), que muestre en vivo el porcentaje de uso (p.ej.
  `45%`), actualizada a partir del snapshot de límites y de los eventos
  `limits-updated`.
- Añadir un **ajuste configurable** para elegir qué muestra la etiqueta:
  - `off` (solo icono, comportamiento actual),
  - `session` (porcentaje de la sesión de 5h),
  - `week` (porcentaje de la semana),
  - `max` (el mayor entre sesión y semana),
  persistido en la tabla `meta` y expuesto vía comandos Tauri (patrón
  `get_*`/`set_*`), con un control en el popover.
- Elegir automáticamente **qué medidor "manda"** cuando el modo es `max`, y
  mostrar un estado no numérico (p.ej. `–`) cuando los límites no están
  disponibles, sin dejar texto obsoleto.
- Añadir en el popover una **ayuda breve** que explique cómo priorizar el icono
  en macOS (⌘-arrastrar para reposicionarlo), reconociendo la limitación del
  sistema.

## Capabilities

### New Capabilities
- `menubar-badge`: etiqueta de texto configurable junto al icono del tray que
  muestra el uso en vivo, su ajuste de configuración persistente, su
  actualización por eventos y la orientación al usuario sobre cómo priorizar el
  icono en macOS.

### Modified Capabilities
<!-- El popover y el icono persistente (capability `menubar-popover`) no cambian
     su comportamiento: la etiqueta es una propiedad adicional del mismo TrayIcon
     y no altera el popover, el menú contextual ni la política de activación. -->

## Impact

- **Rust (`packages/app/src-tauri`)**:
  - `src/lib.rs` — `build_tray()`: guardar un handle del `TrayIcon` en el estado
    de la app y aplicar `set_title` según el ajuste y el último snapshot.
  - Suscripción/handler para actualizar el título cuando llega un nuevo snapshot
    de límites (reutilizando el flujo `limits-updated`).
  - `src/limits/mod.rs` (o módulo de settings): nuevos comandos
    `get_menubar_badge_mode` / `set_menubar_badge_mode` sobre la tabla `meta`.
  - `src/db/mod.rs` — reutiliza `meta_get`/`meta_set`; nueva clave de meta.
- **React (`packages/app/src`)**:
  - `app/Popover.tsx` — control de selección del modo de la etiqueta y ayuda de
    reposicionamiento (junto al `AlertsMuteToggle` existente).
- **Sin cambios de dependencias**: `TrayIcon::set_title` ya está disponible con
  la feature `tray-icon` de Tauri v2 en uso.
