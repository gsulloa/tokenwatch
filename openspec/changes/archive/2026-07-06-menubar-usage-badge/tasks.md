## 1. Backend: setting del modo de la etiqueta

- [x] 1.1 Definir la clave `meta` `menubar_badge_mode` con valores `off|session|week|max` y default `off`; documentar el contrato de valores en el módulo de settings.
- [x] 1.2 Añadir helpers de lectura/escritura sobre `meta` (reutilizando `meta_get`/`meta_set` en `src/db/mod.rs`), con parseo/validación del valor a un enum `BadgeMode`.
- [x] 1.3 Exponer los comandos Tauri `get_menubar_badge_mode` y `set_menubar_badge_mode` (patrón de `get_alerts_muted`/`set_alerts_muted`) y registrarlos en el `invoke_handler`.

## 2. Backend: handle del TrayIcon y formateo del texto

- [x] 2.1 En `build_tray()` (`src/lib.rs`), dejar de descartar el `TrayIcon` y guardarlo en el estado gestionado de la app (`app.manage(...)` o campo del struct de estado) para poder mutarlo luego.
- [x] 2.2 Implementar una función `format_badge(mode, snapshot) -> Option<String>` que devuelva `None` para `off`, `"N%"` para `session`/`week`/`max`, y `"–"` cuando el snapshot no está disponible; `max` toma el mayor entre sesión y semana.
- [x] 2.3 Implementar `apply_badge(app)` que lea el modo actual y el último snapshot conocido y llame `tray.set_title(...)` en consecuencia.

## 3. Backend: aplicar y actualizar la etiqueta

- [x] 3.1 Llamar `apply_badge` al construir el tray usando el último snapshot conocido (si existe) para fijar el valor inicial.
- [x] 3.2 Enganchar `apply_badge` al flujo `limits-updated` para recalcular el título ante cada snapshot nuevo, sin introducir polling adicional.
- [x] 3.3 En `set_menubar_badge_mode`, re-aplicar la etiqueta inmediatamente con el último snapshot tras persistir el nuevo modo.

## 4. Frontend: control en el popover

- [x] 4.1 Leer DESIGN.md y ajustar el control (selector/segmented) al sistema de diseño antes de implementar.
- [x] 4.2 En `src/app/Popover.tsx`, añadir junto a `AlertsMuteToggle` un selector del modo de etiqueta que lea el valor inicial con `get_menubar_badge_mode` y lo escriba con `set_menubar_badge_mode` vía `safeTauriInvoke`.
- [x] 4.3 Añadir el texto de ayuda de reposicionamiento (⌘-arrastrar), sin afirmar que la app puede forzar la visibilidad del ítem.

## 5. Verificación

- [x] 5.1 Pruebas unitarias de `format_badge` (cada modo, `max`, snapshot no disponible → `–`, `off` → sin texto).
- [x] 5.2 `pnpm typecheck && pnpm lint && pnpm test:run` y `cargo fmt/clippy/test` en verde.
- [ ] 5.3 QA en macOS en la barra real: verificar texto en modos `session`/`week`/`max`, cambio en vivo, estado `–` sin datos, persistencia tras reinicio, y apariencia en barra clara/oscura (con y sin notch). *(pendiente: requiere verificación manual en el equipo — no se puede automatizar headless)*
