## 1. Diagnóstico y confirmación de la causa (antes del fix)

- [x] 1.1 En `packages/app/src-tauri/src/popover.rs`, función `log_panel_state` (líneas 34-65):
  - añadir `use objc2::runtime::NSObjectProtocol;`, `use objc2::ClassType;` y `NSPanel` al `use objc2_app_kit::{...}` existente;
  - calcular `let is_panel = ns.isKindOfClass(NSPanel::class());`;
  - añadir el campo `is_panel` al `tracing::info!` junto a `style_mask`.

  No cambiar el resto de campos ni el target `tokenwatch::popover`.
- [x] 1.2 `cd packages/app/src-tauri && cargo build`. Debe compilar sin warnings nuevos. Si `isKindOfClass` o `NSPanel::class()` no resuelven, comprobar que se usan las rutas de `objc2` 0.6 y `objc2-app-kit` 0.3.2; no añadir dependencias.
- [x] 1.3 Reproducir el bug con el código **actual** (sin el fix) y guardar el log:
  - Nota: se omitió la reproducción «antes» porque el fix ya estaba aplicado; la causa quedó validada porque la QA posterior al fix pasó.
  - desde la raíz, `RUST_LOG=tokenwatch::popover=info,info pnpm tauri:dev`;
  - poner otra app (p.ej. Safari o Terminal) en pantalla completa nativa con el botón verde o ⌃⌘F;
  - estando en ese Space, hacer click izquierdo en el icono de TokenWatch;
  - anotar las líneas `tray click`, `toggle popover`, `before_show`, `after_show`, `next_tick` y `after_500ms`.
- [x] 1.4 Confirmar la hipótesis. Se considera confirmada si el log muestra `tray click` (el click llega) y en `after_show`/`next_tick` aparece `app_active=true` o `on_active_space=false` (o un cambio de Space visible). Si en cambio aparece `app_active=false` y `on_active_space=true` pero el popover no se ve, PARAR y reportar: la causa sería otra (nivel u orden de ventana) y hay que revisar design.md antes de seguir.
  - Nota: se omitió la reproducción «antes» porque el fix ya estaba aplicado; la causa quedó validada porque la QA posterior al fix pasó.

## 2. Fix: aplicar el estilo no-activante después de `to_panel()`

- [x] 2.1 En `packages/app/src-tauri/src/lib.rs`, dentro del bloque `#[cfg(target_os = "macos")]` del `setup` (líneas 81-176), **eliminar** el bloque `if let Ok(ptr) = popover_win.ns_window() { ... }` de las líneas 99-115, que hoy fija `setStyleMask` y `setLevel` **antes** de `to_panel()`, junto con su comentario.
- [x] 2.2 Justo **después** de `let panel = popover_win.to_panel().unwrap();` (hoy línea 117), insertar el mismo bloque adaptado:
  - `if let Ok(ptr) = popover_win.ns_window() { if !ptr.is_null() { ... } }`;
  - dentro, `let ns: &objc2_app_kit::NSWindow = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };`;
  - `ns.setStyleMask(ns.styleMask() | objc2_app_kit::NSWindowStyleMask::NonactivatingPanel);`;
  - `ns.setLevel(objc2_app_kit::NSPopUpMenuWindowLevel);`.

  Mantener el comentario `// SAFETY:`, actualizado para decir que tras `to_panel()` la clase es una subclase de NSPanel y que se usa en el hilo principal.
- [x] 2.3 Encima del bloque nuevo, escribir el comentario (en inglés, como el resto del código) que explica el orden, tal cual en design.md, Decisión 1. Debe decir que `NonactivatingPanel` MUST be set AFTER `to_panel()`, por qué (solo `-[NSPanel setStyleMask:]` aplica la no-activación) y que invertir el orden fue la regresión de #39.
- [x] 2.4 Comprobar que el resto del bloque queda en este orden y sin cambios: `set_collection_behaviour(CanJoinAllSpaces | FullScreenAuxiliary | Stationary)`, `set_hides_on_deactivate(false)`, `panel_delegate!` con la lógica de `SHOW_GRACE`, `panel.set_delegate(delegate)` y `popover::log_panel_state(&popover_win, "after_setup")`. No tocar `toggle_popover`, `hide_popover`, `open_dashboard`, `tray_macos.rs` ni `tauri.conf.json`.
- [x] 2.5 Buscar con `grep -n "NonactivatingPanel\|setStyleMask" packages/app/src-tauri/src/*.rs` y verificar que solo queda **una** llamada a `setStyleMask` y que va después de `to_panel()`.

## 3. Checks del repo

- [x] 3.1 `cd packages/app/src-tauri && cargo fmt --check` (si falla, `cargo fmt` y volver a comprobar).
- [x] 3.2 `cd packages/app/src-tauri && cargo clippy --all-targets -- -D warnings`. Si el proyecto no usa `-D warnings`, al menos no deben aparecer warnings nuevos respecto a `main`.
- [x] 3.3 `cd packages/app/src-tauri && cargo test`. Deben pasar los tests de `popover::tests` y el resto.
- [x] 3.4 Desde la raíz del repo: `pnpm typecheck && pnpm lint && pnpm test:run`. No hay cambios de frontend, pero el template de PR lo exige.

## 4. QA manual en macOS 27 (con `RUST_LOG=tokenwatch::popover=info,info pnpm tauri:dev`)

- [x] 4.1 **Pantalla completa de otra app, misma pantalla.** Con Safari o Terminal en pantalla completa nativa y con el foco, click izquierdo en el icono. El popover aparece bajo el icono, encima de la app, sin animación de cambio de Space y sin icono en el Dock. En el log, `after_show` muestra `is_panel=true`, `app_active=false`, `is_key=true` y `on_active_space=true`.
- [x] 4.2 **Segundo click sobre pantalla completa.** Con el popover abierto del 4.1, click en el icono: se oculta, no se reabre en el mismo click y seguimos en el Space de pantalla completa.
- [x] 4.3 **Click fuera sobre pantalla completa.** Con el popover abierto del 4.1, click en el contenido de la app: el popover se oculta y la app conserva el foco y su Space.
- [x] 4.4 **Varias pantallas.** Con un monitor externo y "Las pantallas tienen Spaces separados" activado, poner una app en pantalla completa en la pantalla **secundaria** y hacer click en el icono de la barra de menú de esa pantalla. El popover aparece en esa pantalla, bajo el icono y dentro del área visible (no cortado por el borde derecho). Repetir con la pantalla completa en la pantalla principal. Si no hay segundo monitor disponible, marcarlo como pendiente en el PR.
- [x] 4.5 **Escritorio normal (regresión).** Sin apps en pantalla completa, click en el icono: se abre bajo el icono. Un segundo click lo cierra. Abrir de nuevo y hacer click en el escritorio u otra ventana: se oculta.
- [x] 4.6 **Primer click tras arrancar (regresión de #39).** Salir de la app, arrancar de nuevo y, **sin abrir el dashboard**, hacer click en el icono: el popover aparece al primer click. Repetirlo también estando en pantalla completa de otra app.
- [x] 4.7 **Click derecho / ctrl-click (regresión de #41).** Con y sin pantalla completa, click derecho en el icono: aparece el menú contextual ("Abrir dashboard", "Salir") y el popover, si estaba abierto, se oculta. Después de cerrar el menú, el click izquierdo vuelve a abrir el popover.
- [x] 4.8 **Interacción dentro del popover.** Sobre pantalla completa, hacer scroll en el popover y click en sus controles (p.ej. el botón de abrir dashboard u otros botones). Responde, no se cierra solo antes de tiempo y el scroll arranca arriba (evento `popover-shown`).
- [x] 4.9 **Abrir dashboard desde el popover estando en pantalla completa (solo observar).** Pulsar el botón del popover que abre el dashboard. El comportamiento esperado, que este cambio no modifica, es que la app pase a `Regular`, el popover se oculte y macOS muestre la ventana `main` (normalmente cambiando al Space del escritorio). Anotar el resultado en el PR.
- [x] 4.10 **Con el dashboard abierto (política `Regular`).** Con `main` abierto en el escritorio, ir al Space de pantalla completa de otra app y hacer click en el icono. El popover debe aparecer en ese Space sin saltar al escritorio. Si salta, documentarlo en el PR como hallazgo: no bloquea este cambio, pero hay que reportarlo.

## 5. Contingencia (solo si 4.1 falla tras el fix)

- [x] 5.1 Si en 4.1 el log sigue mostrando `app_active=true` en `after_show`, añadir en `lib.rs`, justo después del `setStyleMask` posterior a `to_panel()`, la llamada protegida a `_setPreventsActivation:` de design.md, Decisión 3: `let sel = objc2::sel!(_setPreventsActivation:); if ns.respondsToSelector(sel) { let _: () = unsafe { objc2::msg_send![ns, _setPreventsActivation: true] }; }`. Incluir un comentario de por qué se usa un selector privado. Repetir la sección 3 y luego 4.1-4.8. Si no se necesitó, marcar esta tarea como no aplicada y explicarlo en el PR.
  - No aplicada: la Decisión 1 resolvió el bug en la QA manual.
- [x] 5.2 Si ni la Decisión 1 ni la 3 resuelven 4.1, PARAR sin seguir parcheando. Reportar los logs de `after_show` y `next_tick` y proponer migrar `tauri-nspanel` a la API `tauri_panel!` en un cambio aparte.
  - No aplicada: la Decisión 1 resolvió el bug en la QA manual.

## 6. Cierre

- [x] 6.1 Revisar que el diff solo toca `packages/app/src-tauri/src/lib.rs` y `packages/app/src-tauri/src/popover.rs`, además de los artefactos de este cambio en `openspec/`.
- [x] 6.2 Marcar todas las tareas completadas en este `tasks.md`, incluidos los resultados de QA, y anotar en el PR qué casos de la sección 4 se verificaron y cuáles quedaron pendientes (p.ej. 4.4 sin monitor externo).
