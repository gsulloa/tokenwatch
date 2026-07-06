## Context

El icono del tray se construye hoy en `packages/app/src-tauri/src/lib.rs`
(`build_tray()`, líneas 187–285) con `TrayIconBuilder`, un PNG monocromo en modo
template (`.icon_as_template(true)`) y **sin texto** (`set_title` no se llama). El
`TrayIcon` resultante se descarta (`let _tray = ...`), por lo que actualmente no
hay un handle para modificarlo después del arranque.

Los datos de uso ya fluyen: el backend consulta límites (`query_limits`), emite
`limits-updated` cuando hay un snapshot nuevo, y el popover (`src/app/Popover.tsx`)
ya consume esos porcentajes. Existe infraestructura de ajustes: tabla `meta`
(clave/valor) en `src/db/mod.rs` con `meta_get`/`meta_set`, y el patrón de
comandos Tauri `get_alerts_muted`/`set_alerts_muted` con un toggle en el popover
(`AlertsMuteToggle`).

**Restricción central de la plataforma:** en macOS una app **no** puede fijar la
prioridad ni el orden de su `NSStatusItem` respecto a los ítems de otras apps. El
orden lo decide el usuario (⌘-arrastrar, persistido por el sistema) y el espacio
disponible; en Macs con notch los ítems sobrantes se ocultan. No existe API para
"forzar" visibilidad. Por tanto el diseño no intenta controlar el orden: lleva la
información al propio ítem (texto) y orienta al usuario a reposicionarlo.

## Goals / Non-Goals

**Goals:**
- Mostrar el porcentaje de uso como **texto junto al icono** del tray, en vivo.
- Hacer el contenido de la etiqueta **configurable y persistente** (`off` /
  `session` / `week` / `max`), con control en el popover.
- Actualizar la etiqueta a partir del mismo flujo que ya alimenta el popover
  (snapshot de límites + evento `limits-updated`), sin polling adicional.
- Degradar con claridad cuando no hay datos (mostrar `–`, no un valor obsoleto).
- Explicar al usuario, dentro de la app, cómo priorizar el icono en macOS.

**Non-Goals:**
- Forzar el orden/visibilidad del ítem en la barra de menú (imposible por API).
- Gestionar overflow/notch o integrar con managers de terceros (Bartender/Ice).
- Cambiar el popover, el menú contextual o la política de activación
  (`Accessory`/`Regular`).
- Mostrar en la etiqueta datos por proyecto o tokens absolutos (solo porcentaje
  de límites en esta iteración).

## Decisions

### 1. Guardar un handle del `TrayIcon` en el estado de la app
Hoy el tray se descarta. Cambiaremos `build_tray()` para **conservar el
`TrayIcon`** en el estado gestionado de Tauri (p.ej. un campo en el struct de
estado existente, o `app.manage(...)`), de modo que el handler de
`limits-updated` pueda llamar `tray.set_title(Some(texto))`.
- **Alternativa considerada:** recrear el tray en cada actualización → descartada:
  provoca parpadeo, pierde posición y es innecesario; `set_title` es en sitio.

### 2. Renderizar el texto con `TrayIcon::set_title`
En macOS `set_title` coloca el texto junto al icono en la barra de menú (respeta
el modo template del icono). El formato será compacto: `"45%"`. Cuando el modo es
`off` → `set_title(None)` (solo icono, estado actual). Cuando no hay snapshot o
los límites no están disponibles → `"–"`.
- **Alternativa:** dibujar el número dentro del PNG del icono → descartada: exige
  render dinámico de imagen por cada cambio; `set_title` es nativo y más simple.

### 3. Fuente de datos: reutilizar el flujo de límites existente
El título se recalcula en dos momentos: (a) al **construir el tray** con el último
snapshot conocido (si existe) y (b) cuando el backend **emite `limits-updated`**.
El backend ya produce ese evento; añadiremos un listener/hook en Rust que, ante un
snapshot nuevo, formatee el texto según el modo actual y llame `set_title`.
- **Alternativa:** un timer propio del tray → descartada: duplicaría el polling ya
  existente y podría desincronizarse del popover.

### 4. Modo de la etiqueta como setting en `meta`
Nueva clave `meta` (p.ej. `menubar_badge_mode`) con valores
`off|session|week|max` (default `off` para no cambiar el comportamiento de quien
ya usa la app). Comandos Tauri `get_menubar_badge_mode` / `set_menubar_badge_mode`
siguiendo el patrón de `get_alerts_muted`/`set_alerts_muted`. Al escribir el
setting, el backend **re-aplica** el título inmediatamente con el último snapshot.
- **`max`**: toma el mayor entre el % de sesión y el % de semana (el que esté más
  cerca de agotarse), que suele ser el número que el usuario quiere vigilar.
- **Alternativa:** archivo de config aparte → descartada: `meta` ya es el
  mecanismo estándar de settings del proyecto.

### 5. UI del control en el popover
Añadir junto al `AlertsMuteToggle` un selector (segmented/select) para el modo de
la etiqueta, leyendo el valor inicial con `get_menubar_badge_mode` y escribiendo
con `set_menubar_badge_mode` (mismo patrón que el toggle actual). Debe respetar
DESIGN.md (leerlo antes de implementar la UI). Incluir un texto de ayuda breve:
"Para que no se oculte, arrastra el icono con ⌘ hacia la izquierda de la barra".

## Risks / Trade-offs

- [Un ítem con texto es **más ancho** y podría desbordar antes en barras llenas]
  → Mitigación: texto compacto (`45%`); el valor está en el propio ítem, así que
  incluso parcialmente visible aporta información; default `off`.
- [La etiqueta **no impide** que macOS oculte el ítem] → Mitigación: es una
  limitación de plataforma explícita en el proposal; la app orienta a
  reposicionar (⌘-arrastrar). No se promete "priorizar" el orden.
- [Datos no disponibles dejarían un número **obsoleto**] → Mitigación: mostrar `–`
  cuando el snapshot no está disponible; nunca conservar el último % como si fuera
  actual.
- [Concurrencia al mutar el `TrayIcon` desde el handler de eventos] → Mitigación:
  acceder al handle vía el estado gestionado de Tauri (thread-safe) y formatear en
  el hilo del evento; `set_title` es una operación ligera.
- [Diferencias de render de `set_title` entre versiones de macOS] → Mitigación:
  validar en QA en la barra real (claro/oscuro, con/sin notch).
