## 1. Diagnóstico del popover (antes de los fixes de la sección 5)

- [x] 1.1 Crear `packages/app/src-tauri/src/popover.rs` y registrarlo en `lib.rs` con `pub mod popover;`. Añadir `#[cfg(target_os = "macos")] pub fn log_panel_state(win: &tauri::WebviewWindow, stage: &str)`, que obtiene `NSWindow` desde `win.ns_window()` (cast a `*const objc2_app_kit::NSWindow`) y `NSApplication::sharedApplication(MainThreadMarker::new()?)`. Debe registrar `isVisible`, `isKeyWindow`, `isOnActiveSpace`, `styleMask().0`, `level()`, `hidesOnDeactivate()` y `isActive` con `tracing::info!(target: "tokenwatch::popover", ...)`.
- [x] 1.2 En `on_tray_icon_event` (`lib.rs` ~:231-284), registrar:
  - al recibir el click, la posición y `panel.is_visible()`;
  - `log_panel_state` en `before_show` y `after_show`;
  - en la siguiente vuelta (`next_tick`) vía `app.run_on_main_thread`;
  - a los 500 ms (`after_500ms`) vía `tauri::async_runtime::spawn` + sleep + `run_on_main_thread`.
- [x] 1.3 En el listener del delegado `window_did_resign_key` (`lib.rs` ~:113-121), registrar los ms transcurridos desde el último show, usando `PopoverTiming.shown_at` (ver 5.4).
- [x] 1.4 Al final del bloque de conversión a NSPanel en `setup`, llamar a `log_panel_state(.., "after_setup")`.
- [ ] 1.5 Protocolo de reproducción manual:
  1. Cerrar el TokenWatch instalado.
  2. Ejecutar `cd packages/app && RUST_LOG=info pnpm tauri dev`.
  3. Sin abrir el dashboard, hacer click izquierdo en el tray 3 veces.
  4. Abrir y cerrar el dashboard, y volver a hacer click en el tray.
  5. Repetir el click con otra app en pantalla completa.
  6. Anotar en `design.md` (Open Questions) qué hipótesis confirma cada caso:
     - no aparece la línea de click → H4;
     - `after_show isVisible=false` → H1/H1b;
     - resign en menos de 300 ms → H2;
     - `isOnActiveSpace=false` → H5.

## 2. Parseo de límites (`packages/app/src-tauri/src/limits/mod.rs`)

- [x] 2.1 `ApiResponse`: cambiar a `limits: Option<Vec<serde_json::Value>>`, sin `#[serde(default)]`, de modo que acepte tanto el campo ausente como `null`.
- [x] 2.2 `ApiTopLevel`: hacer opcionales ambos campos, `utilization: Option<f64>` y `resets_at: Option<String>`. Añadir `fn top_level_window(t: Option<ApiTopLevel>, label: Option<&str>) -> Option<Window>`, que devuelve `None` si falta alguno de los dos.
- [x] 2.3 Reescribir `parse_api_response` como `fn parse_api_response(api: ApiResponse, now: DateTime<Utc>) -> Result<LimitsSnapshot, String>`:
  - deserializar cada fila con `serde_json::from_value::<ApiLimit>(v).ok()`;
  - combinar por ventana según D1 (`session = limits.session ?? five_hour`, `weekly = limits.weekly_all ?? seven_day`, scoped o bien Opus/Sonnet top-level);
  - descartar la sesión si `resets_at < now`;
  - si la sesión queda `None`, escribir un `tracing::info!` con los `kind`s vistos y si había `five_hour`;
  - asignar `fetched_at = now.to_rfc3339()`.
- [x] 2.4 Actualizar la llamada en `fetch_usage` a `parse_api_response(api, Utc::now())`.
- [x] 2.5 Tests en `mod tests`. Helper `fn parse(json: &str) -> LimitsSnapshot` con `now = 2026-11-07T12:00:00Z` y `resets_at` futuros. Casos:
  - `parse_limits_with_session_row`
  - `parse_falls_back_to_five_hour_when_limits_lack_session`
  - `parse_without_session_anywhere_keeps_weekly`, con `five_hour` ausente y también con `null`
  - `parse_five_hour_with_null_fields_does_not_fail`
  - `parse_limits_null_does_not_fail`
  - `parse_weekly_scoped_rows_produce_labelled_windows` (Fable)
  - `limits_session_wins_over_five_hour`
  - `expired_session_is_dropped`
  - `malformed_row_is_skipped` (`percent: "abc"`)
  - `scoped_rows_suppress_top_level_opus_sonnet`
  - `empty_limits_uses_top_level`

## 3. Etiqueta de la barra de menú (`packages/app/src-tauri/src/menubar.rs`)

- [x] 3.1 Añadir `const WEEK_FALLBACK_PREFIX: &str = "W ";` y el helper `fn pct(p: f64) -> String`. En `badge_text`, el modo `Session` queda así:
  - `(Some(s), _)` → `pct(s)`
  - `(None, Some(w))` → `format!("{WEEK_FALLBACK_PREFIX}{}", pct(w))`
  - `(None, None)` → `"–"`

  Actualizar también el doc comment de `format_badge`.
- [x] 3.2 Reemplazar el test `missing_window_in_ok_snapshot_shows_dash` por:
  - `session_mode_falls_back_to_weekly_with_prefix` (`"W 56%"`)
  - `session_mode_without_any_window_shows_dash`
  - `week_mode_missing_weekly_shows_dash`
  - `max_mode_never_uses_prefix` (`"56%"`)

## 4. Popover: estado "Sin sesión activa" (frontend)

- [x] 4.1 Leer `DESIGN.md`. En `packages/app/src/features/limits/LimitsSection.tsx`, añadir un componente local `InactiveSessionRow`:
  - misma cabecera que `LimitGauge`: "Sesión 5h" a la izquierda;
  - "Sin sesión activa" a la derecha en `var(--text-muted)` 12px;
  - `role="status"`.

  Renderizarlo en lugar del gauge de sesión cuando `!snapshot.session && snapshot.weekly`.
- [x] 4.2 Crear `packages/app/src/features/limits/LimitsSection.test.tsx` con tres casos:
  - snapshot `ok` con `session: null` y semana al 56 → aparece "Sin sesión activa" y el gauge semanal;
  - con ambas ventanas → no aparece el texto;
  - con ninguna → "Sin datos de límites disponibles".

## 5. Popover NSPanel (`packages/app/src-tauri/src/lib.rs`, `src/popover.rs`)

- [x] 5.1 Reemplazar `panel.set_style_mask(NSWindowStyleMaskNonActivatingPanel)` por un OR tipado sobre la máscara existente. Obtener `ns_window()` del `popover_win` antes de `to_panel()`:

  ```rust
  ns.setStyleMask(ns.styleMask() | objc2_app_kit::NSWindowStyleMask::NonactivatingPanel)
  ```
- [x] 5.2 Reemplazar `panel.set_level(101)` por `ns.setLevel(objc2_app_kit::NSPopUpMenuWindowLevel)` y eliminar las constantes `i32` locales.
- [x] 5.3 Añadir `panel.set_hides_on_deactivate(false);` después de configurar el `collectionBehavior`.
- [x] 5.4 En `popover.rs`, añadir:
  - `pub const SHOW_GRACE: Duration = Duration::from_millis(300)`
  - `pub const RESIGN_CLICK_DEBOUNCE: Duration = Duration::from_millis(250)`
  - `#[derive(Default)] pub struct PopoverTiming { shown_at: Mutex<Option<Instant>>, hidden_by_resign_at: Mutex<Option<Instant>> }`
  - `pub fn within(since: Option<Instant>, now: Instant, window: Duration) -> bool`

  Registrar el estado con `app.manage(popover::PopoverTiming::default())` en `setup`, antes de construir el tray.
- [x] 5.5 Listener `window_did_resign_key`:
  - si `within(shown_at, now, SHOW_GRACE)`: log `"resign ignored"` y `app_handle.run_on_main_thread(|| panel.make_key_window())`;
  - si no: `order_out(None)` y `hidden_by_resign_at = Some(now)`.
- [x] 5.6 Handler del click izquierdo:
  - si `panel.is_visible()` → `order_out`;
  - si no, y `within(hidden_by_resign_at, now, RESIGN_CLICK_DEBOUNCE)` → log `"dismiss click"`, limpiar y no reabrir;
  - en otro caso → `position_popover_near_click` y luego `app.run_on_main_thread(move || { shown_at = now; panel.show(); if !panel.is_visible() { panel.order_front_regardless(); panel.make_key_window(); log } emit("popover-shown") })`.
- [x] 5.7 Tests unitarios de `within` en `popover.rs`: con `None` → false; dentro de la ventana → true; en el límite y fuera → false.
- [x] 5.8 Mantener todo bajo `#[cfg(target_os = "macos")]`. La rama no-macOS queda sin cambios.

## 6. Persistencia de ventanas (`packages/app/src-tauri/src/lib.rs`)

- [x] 6.1 Cambiar el registro del plugin por:

  ```rust
  tauri_plugin_window_state::Builder::default()
      .with_denylist(&["popover"])
      .with_state_flags(StateFlags::all() - StateFlags::FULLSCREEN - StateFlags::VISIBLE)
      .build()
  ```

  Importar `tauri_plugin_window_state::StateFlags` y añadir un comentario con el motivo.

## 7. Verificación

- [x] 7.1 `cd packages/app/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- [x] 7.2 `cd packages/app && pnpm typecheck && pnpm lint && pnpm test:run`
- [ ] 7.3 QA manual del bug 1:
  - en modo `session` sin sesión activa, el título muestra `W N%`;
  - con sesión activa, muestra `N%`;
  - el popover muestra "Sin sesión activa";
  - el log de `kind`s confirma la forma real de la API.
- [ ] 7.4 QA manual del bug 2, con el protocolo 1.5 ya con los fixes:
  - (a) primer click sin haber abierto el dashboard → el popover aparece;
  - (b) click fuera → se oculta;
  - (c) click en el icono con el popover abierto → se cierra y no se reabre;
  - (d) sobre una app en pantalla completa → aparece sin cambiar de Space ni mostrar el Dock;
  - (e) tras abrir y cerrar el dashboard → sigue funcionando;
  - (f) con un `.window-state.json` heredado con `main.fullscreen: true` → no aparece ninguna ventana al arrancar.
- [ ] 7.5 Con la causa confirmada, bajar a `debug` las trazas por click (`next_tick`, `after_500ms`), mantener en `info` el log de `"resign ignored"` y documentar la causa en `design.md`.
- [x] 7.6 `openspec validate fix-session-limit-and-tray-popover --strict`
