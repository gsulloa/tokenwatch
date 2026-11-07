## Context

**Bug 1: la etiqueta muestra `–`.**

`parse_api_response` (`packages/app/src-tauri/src/limits/mod.rs:358-447`) usa `limits[]` si no está vacío y, solo si está vacío, cae a los campos top-level (`:408-416`). La API ahora devuelve `limits[]` con `weekly_all` y `weekly_scoped` pero sin `session`. `five_hour` es opcional: solo está presente mientras la ventana de 5h está activa.

Los datos locales del usuario lo confirman. En la tabla `meta`, `alert_weekly` y `alert_weekly_scoped:Fable` están al día, pero `alert_session` está congelado desde 2026-10-31.

Hay además una fragilidad latente: `ApiTopLevel { utilization: f64, resets_at: String }` no es opcional, y `#[serde(default)] limits` no acepta `null`. Con cualquiera de esos dos casos falla todo el parseo.

La etiqueta (`menubar.rs:64-95`) en modo `session` devuelve `–` cuando la sesión es `None`. El usuario tiene `menubar_badge_mode = session`.

**Bug 2: el popover no abre hasta abrir el dashboard.**

El popover es un NSPanel creado con `tauri-nspanel` (rama v2, `18ffb9a`) en `lib.rs:65-124`. Su configuración:
- `set_style_mask(NonActivatingPanel)` **reemplaza** toda la máscara;
- `set_level(101)` recibe un `i32`, igual que la máscara;
- el delegado `window_did_resign_key` llama a `order_out`;
- nunca se fija `hidesOnDeactivate`, que en NSPanel vale YES por defecto.

La app arranca en modo `Accessory`. Abrir el dashboard la pasa a `Regular` y le hace `set_focus`, lo que la activa una vez; a partir de ahí el popover funciona.

`tauri_plugin_window_state::Builder::default()` restaura todos los flags de todas las ventanas. El estado guardado del usuario tiene `main.fullscreen = true` y `popover` en 720x960.

El código no cambió desde julio; lo que cambió es macOS, que pasó a 27.0.1. La causa raíz no se puede confirmar estáticamente.

## Goals / Non-Goals

**Goals:**
- Que la etiqueta vuelva a dar un valor útil cuando no hay sesión activa: % semanal con prefijo `W `.
- Que el parseo sea robusto ante la forma nueva y variante de `/api/oauth/usage`, con tests que cubran cada forma.
- Que el popover abra desde el primer click sin activar la app y sin perder el comportamiento sobre Spaces en pantalla completa (ver cambio archivado `2026-07-06-popover-fullscreen-space`).
- Dejar trazas que permitan confirmar la causa del bug 2 en la máquina del usuario.

**Non-Goals:**
- Migrar o actualizar `tauri-nspanel`, ni usar APIs privadas (`_setPreventsActivation:`).
- Refrescar el token OAuth.
- Rediseñar los modos de la etiqueta o el popover más allá de "Sin sesión activa".

## Decisions

**D1: Combinar por ventana en lugar de "limits[] o top-level".**
Reglas:
- `session = limits.session ?? five_hour`
- `weekly = limits.weekly_all ?? seven_day`
- `weekly_by_model`: las filas `weekly_scoped` si existe alguna; si no, `seven_day_opus` / `seven_day_sonnet`, para no duplicar Opus.

*Alternativa descartada:* usar siempre top-level para la sesión. No se elige porque `limits[]` trae `resets_at` más preciso y es la forma que Claude Code clasifica por `kind`.

**D2: Parseo tolerante a nivel de tipo.**
- `ApiResponse.limits: Option<Vec<serde_json::Value>>`. Cada fila se deserializa con `serde_json::from_value::<ApiLimit>(v).ok()`, así que una fila mala se descarta sin tumbar la respuesta.
- `ApiTopLevel { utilization: Option<f64>, resets_at: Option<String> }`, con un helper `top_level_window(...) -> Option<Window>`.
- `parse_api_response(api, now: DateTime<Utc>)` descarta la sesión si `resets_at < now`. Recibir `now` como parámetro hace la función determinista en tests.
- Cuando la sesión queda `None`, se escribe un `tracing::info!` con los `kind`s vistos y si `five_hour` estaba presente, sin datos personales.

*Alternativa descartada:* un `deserialize_with` personalizado para `null`. `Option` es más simple y claro.

**D3: Respaldo semanal en modo `session`, con prefijo `W `.**
- El prefijo aparece solo cuando se muestra una ventana sustituta, para no hacerla pasar por la sesión. La constante `WEEK_FALLBACK_PREFIX` vive en `menubar.rs`.
- `week` y `max` no cambian.
- `–` solo aparece con snapshot ausente o no disponible, o sin ninguna ventana aplicable.

*Alternativas descartadas:* mostrar `0%`, que engaña porque la semana puede estar al 90 %, o eliminar el modo `session`, que es un cambio de configuración para el usuario. El usuario eligió el respaldo semanal.

**D4: "Sin sesión activa" en el popover sin tocar `LimitGauge`.**
`LimitsSection.tsx` incluye un componente local `InactiveSessionRow`, con la misma cabecera que el gauge: "Sesión 5h" a la izquierda y "Sin sesión activa" en `var(--text-muted)` a la derecha. Se renderiza cuando `!snapshot.session && snapshot.weekly`. Sigue DESIGN.md y no toca `useLimits` ni los tipos.

**D5: Primero diagnóstico, luego un conjunto de fixes de bajo riesgo que cubre las hipótesis principales.**
Se mantienen la máscara no-activante, el `collectionBehavior` CanJoinAllSpaces | FullScreenAuxiliary | Stationary y el nivel de popup. Nunca se llama a `NSApp.activate`, porque reintroduciría el salto de Space.

| # | Elemento | Hipótesis |
|---|---|---|
| a | `setStyleMask(styleMask() \| NonactivatingPanel)` con `objc2-app-kit` | La máscara reemplazada pierde bits que macOS 27 necesita; además corrige el ABI `i32` |
| b | `setLevel(NSPopUpMenuWindowLevel)` tipado | Corrige el ABI `i32` |
| c | `panel.set_hides_on_deactivate(false)` | H1b: una app Accessory nunca activada está siempre inactiva y AppKit oculta el panel. Encaja exactamente con "solo funciona tras abrir el dashboard" |
| d | Diferir `panel.show()` con `app.run_on_main_thread` | El status item recupera el key durante su seguimiento del mouse-up |
| e | Verificar en la siguiente vuelta; si `!isVisible`, `order_front_regardless()` + `make_key_window()` una sola vez, con log | Red de seguridad |
| f | Gracia de 300 ms: se ignora un `resign key` tras mostrar y se re-llama `make_key_window()` | H2: un `resign` espurio oculta el panel al instante |
| g | Debounce de 250 ms: si el panel se ocultó por `resign` hace muy poco, el click del tray solo lo cierra | Evita que el mouse-down oculte y el mouse-up reabra |
| h | window-state con `.with_denylist(&["popover"])` y flags `all() - FULLSCREEN - VISIBLE` | H3: restaurar fullscreen o visibilidad de un `main` oculto altera los Spaces; además contradice "Oculto al arrancar" |

Todo se aplica aunque el diagnóstico confirme una sola hipótesis, porque cada elemento es de bajo riesgo y se protegen entre sí.

*Alternativa descartada:* actualizar `tauri-nspanel` a la API objc2 (`tauri_panel!`). Requiere red y es una migración de API; queda como seguimiento si los logs apuntan a la librería.

**D6: Lógica de tiempos pura y testeable en `src/popover.rs`.**
- `SHOW_GRACE` y `RESIGN_CLICK_DEBOUNCE`.
- `PopoverTiming { shown_at, hidden_by_resign_at }`, registrado como estado de Tauri.
- `within(since, now, window) -> bool`, con tests unitarios.
- `log_panel_state(win, stage)` en macOS: lee con objc2 `isVisible`, `isKeyWindow`, `isOnActiveSpace`, `styleMask`, `level`, `hidesOnDeactivate` y `NSApplication::isActive`, y los registra con el target `tokenwatch::popover`.

## Risks / Trade-offs

- [Un click real fuera del popover antes de 300 ms no lo oculta] → El re-key hace que el siguiente click fuera dispare `resign` y lo cierre.
- [El prefijo `W ` alarga el título del tray] → Solo aparece en el respaldo, y es una constante fácil de cambiar.
- [Las trazas `info` añaden ruido al log] → Se bajan a `debug` una vez confirmada la causa (tarea 8.5).
- [Ninguna de las hipótesis es la correcta] → Las trazas de la sección 1 de tareas dicen cuál camino falla. Los siguientes pasos serían actualizar `tauri-nspanel` o usar `_setPreventsActivation:`.
- [`ns_window()` usado fuera del hilo principal] → Todo el acceso ocurre en `setup`, en los callbacks del tray o del delegado, o vía `run_on_main_thread`.

## Migration Plan

- No hay migración de datos. El primer arranque tras actualizar ignora el `fullscreen` y el `visible` guardados de `main` y todo el estado del `popover`.
- Rollback: revertir el commit. El estado de ventanas sigue siendo compatible.

## Open Questions

- Causa raíz confirmada del bug 2. Se completa tras el protocolo de reproducción de la tarea 1.5.
- `kind`s reales que devuelve hoy `limits[]`. Los da el log de D2 en el primer poll.
