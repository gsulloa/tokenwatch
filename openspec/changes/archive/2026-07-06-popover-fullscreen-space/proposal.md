## Why

Cuando el usuario está en una ventana en modo pantalla completa (un Space de fullscreen de macOS), hacer click en el icono del tray no muestra el popover: la ventana se abre en el Space "escritorio" y queda invisible hasta que el usuario cambia de Space manualmente. Un app de barra de menú debe poder desplegarse sobre cualquier Space, incluido el fullscreen activo, o pierde su utilidad principal.

## What Changes

- El popover SHALL configurarse a nivel de `NSWindow` para poder mostrarse sobre el Space activo, incluido cualquier Space en pantalla completa, sin obligar al usuario a cambiar de Space.
- Se ajustará el `collectionBehavior` de la ventana del popover (unir todos los Spaces + auxiliar de pantalla completa) y su nivel de ventana para que aparezca por encima de las ventanas en fullscreen.
- El comportamiento se aplicará una sola vez durante el `setup` (macOS), reutilizando la infraestructura `objc2`/`objc2-app-kit` que el proyecto ya usa para el icono del Dock.
- No cambia el posicionamiento horizontal ni el auto-ocultado por pérdida de foco; solo se garantiza la visibilidad sobre el Space activo.

## Capabilities

### New Capabilities
<!-- Ninguna capacidad nueva; esto refina el comportamiento existente del popover. -->

### Modified Capabilities
- `menubar-popover`: el requisito "Popover desplegable desde el tray" se extiende para exigir que el popover sea visible sobre el Space activo, incluidas las ventanas en pantalla completa.

## Impact

- `packages/app/src-tauri/src/lib.rs`: nueva configuración de `NSWindow` (collection behavior + window level) aplicada a la ventana `popover` en el hook `setup`, en el hilo principal (macOS).
- Sin cambios en dependencias: `objc2` y `objc2-app-kit` ya están en `Cargo.toml`.
- Solo afecta a macOS; comportamiento sin cambios en otras plataformas.
- Sin cambios en el frontend ni en `tauri.conf.json` (la config nativa no es expresable ahí).
