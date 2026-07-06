## 1. Configurar la NSWindow del popover (macOS)

- [x] 1.1 En `packages/app/src-tauri/src/lib.rs`, dentro del hook `setup`, tras obtener la ventana `popover`, añadir una función `#[cfg(target_os = "macos")]` que configure su `NSWindow` en el hilo principal (patrón `run_on_main_thread`, como `set_dock_icon`).
- [x] 1.2 Obtener el `NSWindow` nativo desde `WebviewWindow::ns_window()` y fijar `collectionBehavior |= NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorFullScreenAuxiliary` usando `objc2-app-kit`.
- [x] 1.3 Elevar el nivel de la ventana al equivalente de `NSPopUpMenuWindowLevel` para que se dibuje sobre ventanas en pantalla completa.
- [x] 1.4 Verificar que todo el código nativo queda tras `#[cfg(target_os = "macos")]` y no rompe la compilación en otras plataformas.

## 1b. Escalar a NSPanel no-activante (tras QA fallida sobre fullscreen de otra app)

- [x] 1b.1 Añadir la dependencia `tauri-nspanel` (rama `v2`) a `packages/app/src-tauri/Cargo.toml` y registrar el plugin en el `Builder`.
- [x] 1b.2 En el `setup`, convertir la ventana `popover` en `NSPanel` (`to_panel()`) y fijar `style_mask |= NonActivatingPanel`, `collectionBehavior = CanJoinAllSpaces | FullScreenAuxiliary | Stationary` y nivel `PopUpMenu`.
- [x] 1b.3 Reescribir el show/hide del popover en el handler de click del tray para usar la API del panel (`panel.show()` / `panel.order_out(None)`), preservando el posicionamiento junto al icono.
- [x] 1b.4 Preservar el auto-ocultado al perder foco usando el delegado del panel (`window_did_resign_key`) u equivalente; ocultar también el panel al abrir el dashboard (`open_dashboard`).
- [x] 1b.5 Mantener el código nativo tras `#[cfg(target_os = "macos")]`; el resto de plataformas conserva la `WebviewWindow` normal.

## 2. Validación

- [x] 2.1 `cargo fmt`, `cargo clippy` y `cargo build` pasan sin warnings nuevos en `src-tauri`.
- [x] 2.2 QA manual en macOS: con una app en pantalla completa activa, hacer click en el icono del tray muestra el popover sobre esa ventana sin cambiar de Space.
- [ ] 2.3 QA manual: el popover sigue ocultándose al perder el foco (click fuera) y se sigue posicionando bajo el icono del tray.
- [ ] 2.4 QA manual: en un Space "escritorio" normal el comportamiento del popover no cambia (regresión).
