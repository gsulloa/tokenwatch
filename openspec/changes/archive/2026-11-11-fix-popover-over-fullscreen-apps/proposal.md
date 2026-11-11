## Why

Reporte del usuario: "Si estoy en modo full screen de otra app y presiono el icono de la toolbar, no aparece el dashboard". Si otra app está en pantalla completa nativa de macOS (en su propio Space) y el usuario hace click en el icono de TokenWatch en la barra de menú, el popover no aparece sobre esa app. En julio esto ya funcionaba (cambio archivado `2026-07-06-popover-fullscreen-space`, QA 2.2 aprobada). Dejó de funcionar con el fix de macOS 27 (#39, cambio archivado `2026-11-07-fix-session-limit-and-tray-popover`). Ese fix movió la aplicación del estilo `NSWindowStyleMaskNonactivatingPanel` a un momento anterior a la conversión de la ventana en `NSPanel`. Con ese orden el estilo queda marcado en la máscara, pero el panel deja de ser no-activante: al mostrarse activa la app, y macOS cambia de Space o lo deja fuera del Space de pantalla completa. Una app de barra de menú que no se ve sobre apps en pantalla completa pierde buena parte de su utilidad, y este es un uso muy común (editor o terminal a pantalla completa).

## What Changes

- El estilo no-activante del popover se aplica **después** de `to_panel()`, sobre el objeto que ya es `NSPanel`. Así AppKit pasa por la implementación de `-[NSPanel setStyleMask:]`, que es la que convierte el panel en no-activante de verdad. Se conserva la mejora de #39: se añade el bit a la máscara existente (OR) y no se reemplaza la máscara.
- Un comentario en el código explica por qué el orden importa, para evitar una nueva regresión.
- El diagnóstico existente (`log_panel_state`) se amplía para registrar si la ventana ya es un `NSPanel` y si la app quedó activa tras mostrar el popover. Así se puede confirmar la causa en la máquina del usuario.
- Se conservan sin cambios: `collectionBehavior` (CanJoinAllSpaces | FullScreenAuxiliary | Stationary), el nivel `NSPopUpMenuWindowLevel`, `hidesOnDeactivate = NO`, la gracia de 300 ms ante un `resign key`, el debounce de 250 ms del click y el manejo directo del click del tray de #41.
- Plan de contingencia (solo si la QA demuestra que el reordenamiento no basta): forzar la no-activación con el selector privado `_setPreventsActivation:`, protegido por `respondsToSelector:`. Detalle en design.md.
- No cambia el comportamiento de "Abrir dashboard": el dashboard sigue siendo una ventana normal que, al abrirse, activa la app y lleva al usuario a su Space.

## Capabilities

### New Capabilities
<!-- Ninguna. -->

### Modified Capabilities
- `menubar-popover`: se modifica el requisito "Popover desplegable desde el tray". Ahora exige de forma explícita que mostrar el popover sobre la pantalla completa de **otra** app no active TokenWatch ni cambie de Space, y que el estilo no-activante quede efectivo sobre el `NSPanel`, no solo marcado en la máscara. Se añaden escenarios para la pantalla completa de otra app, varias pantallas y el cierre con un segundo click sobre la pantalla completa.

## Impact

- `packages/app/src-tauri/src/lib.rs`: bloque `#[cfg(target_os = "macos")]` del hook `setup`, donde se convierte la ventana `popover` en `NSPanel`. Solo se reordena y se documenta.
- `packages/app/src-tauri/src/popover.rs`: `log_panel_state` registra además `is_panel` (`isKindOfClass(NSPanel)`).
- Sin dependencias nuevas: `objc2`, `objc2-app-kit` y `tauri-nspanel` (rama v2, `18ffb9a`) ya están en uso.
- Solo afecta a macOS. Sin cambios en el frontend, en `tauri.conf.json` ni en otras plataformas.
- Riesgo de regresión acotado al ciclo de mostrar y ocultar el popover. Se cubre con QA manual en escritorio normal, pantalla completa propia y de otra app, y varias pantallas.
