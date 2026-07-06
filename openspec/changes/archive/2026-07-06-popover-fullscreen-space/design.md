## Context

El popover es una `WebviewWindow` estándar de Tauri (label `popover`) configurada en `tauri.conf.json` con `alwaysOnTop: true`, `decorations: false`, `skipTaskbar: true` y `visible: false`. Se muestra desde el handler de click del tray en `packages/app/src-tauri/src/lib.rs` (`win.show()` + `win.set_focus()`).

En macOS, cuando hay una ventana en pantalla completa, esa ventana ocupa su propio Space. Una `NSWindow` normal —incluso con `alwaysOnTop`— no se dibuja sobre el Space de fullscreen activo: aparece en el Space "escritorio", por lo que el usuario no la ve hasta cambiar de Space. `alwaysOnTop` solo eleva el nivel de la ventana relativo a las ventanas de su propio Space; no le permite unirse al Space de fullscreen.

Para que una ventana se muestre sobre cualquier Space, incluido un fullscreen activo, macOS exige configurar su `NSWindowCollectionBehavior`. El proyecto ya enlaza `objc2` y `objc2-app-kit` (usados en `set_dock_icon`), así que podemos acceder al `NSWindow` nativo sin nuevas dependencias.

## Goals / Non-Goals

**Goals:**
- El popover se muestra sobre el Space activo, incluidas ventanas en pantalla completa, sin cambiar de Space.
- Cambio contenido en macOS, aplicado una sola vez, sin regresiones en el auto-ocultado ni el posicionamiento.

**Non-Goals:**
- Reescribir el popover como `NSPanel` o adoptar el plugin `tauri-nspanel` (mayor superficie de cambio; se evalúa solo si el enfoque nativo resultara insuficiente).
- Cambiar el posicionamiento horizontal, el refresco de datos o el auto-ocultado por foco.
- Comportamiento en Windows/Linux.

## Decisions

**Decisión 1 — Configurar `collectionBehavior` + nivel sobre la `NSWindow` del popover, en lugar de solo `alwaysOnTop`.**
Tras crear la ventana, obtener su handle nativo (`WebviewWindow::ns_window()` → `NSWindow`) y fijar:
- `collectionBehavior |= NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorFullScreenAuxiliary` — permite que la ventana aparezca en todos los Spaces y como auxiliar sobre ventanas en fullscreen.
- Nivel de ventana elevado (equivalente a `NSPopUpMenuWindowLevel` / `kCGPopUpMenuWindowLevel`) para dibujarse por encima de la ventana de fullscreen.

Se aplica una vez en el hook `setup`, en el hilo principal vía `run_on_main_thread` (patrón ya usado por `set_dock_icon`), justo después de obtener la ventana `popover`.

*Alternativa considerada:* `tauri-nspanel` — resuelve el caso de forma robusta pero introduce una dependencia y convierte la ventana en `NSPanel`, con más riesgo de regresión en el ciclo show/hide/focus actual. Se descarta como primera opción.

**Decisión 1b (revisión tras QA) — Escalar a `tauri-nspanel` con panel no-activante.**
La QA manual demostró que el enfoque de `NSWindow` + `collectionBehavior` NO basta cuando **otra** app está en pantalla completa: al hacer `set_focus()`, macOS activa nuestra app Accessory y cambia de Space en vez de dibujar el popover sobre el Space activo. La causa es la *activación* de la app; una `NSWindow` normal no puede volverse key sobre el fullscreen de otra app sin provocar el cambio de Space.

Solución: convertir la ventana `popover` en un `NSPanel` con estilo `NSWindowStyleMask::NonActivatingPanel` mediante el plugin `tauri-nspanel` (rama `v2`). Un panel no-activante puede volverse key y dibujarse sobre el Space activo (incluido el fullscreen de otra app) sin activar nuestra app. Se conserva `collectionBehavior = CanJoinAllSpaces | FullScreenAuxiliary | Stationary` y el nivel `PopUpMenu`. El auto-ocultado por pérdida de foco se mantiene vía el delegado del panel (`window_did_resign_key`) o el evento equivalente.

**Decisión 2 — Mantener `alwaysOnTop` en `tauri.conf.json`.**
No entra en conflicto con la configuración nativa y preserva el comportamiento en plataformas no-macOS. La config nativa (collection behavior / window level) no es expresable en `tauri.conf.json`, por eso vive en Rust.

**Decisión 3 — Envolver todo el código nativo en `#[cfg(target_os = "macos")]`.**
Consistente con el resto de `lib.rs`; sin impacto en otras plataformas.

## Risks / Trade-offs

- [El auto-hide por pérdida de foco podría comportarse distinto sobre un Space de fullscreen] → Verificar en QA que al hacer click fuera del popover (sobre la app en fullscreen) el popover se oculta; el listener `WindowEvent::Focused(false)` no cambia.
- [`CanJoinAllSpaces` hace que la ventana persista visualmente al cambiar de Space mientras esté mostrada] → Aceptable: el popover se oculta al perder foco, por lo que su vida visible es corta; no queda "pegado" entre Spaces.
- [API de `objc2-app-kit` para nivel/collection behavior podría diferir según versión del crate] → Usar las constantes/APIs de la versión ya presente en `Cargo.toml`; validar con `cargo build`/`clippy`.

## Migration Plan

Cambio puramente aditivo en el `setup`; no requiere migración de datos ni de configuración. Rollback = revertir el commit. Sin banderas de features.

## Open Questions

- Ninguna que bloquee la implementación. Si en QA el enfoque nativo no cubre algún caso de fullscreen (p.ej. apps que fuerzan su propio nivel), se reevaluaría `tauri-nspanel` como seguimiento.
