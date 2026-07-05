# Tasks — menubar-limits-popover

## 1. Dependencias y permisos

- [x] 1.1 Añadir `tauri-plugin-notification = "2"` y un cliente HTTP (`reqwest` con features `["rustls-tls","json"]`) a `packages/app/src-tauri/Cargo.toml`; registrar el plugin en `lib.rs` (`.plugin(tauri_plugin_notification::init())`).
- [x] 1.2 Añadir el permiso `notification:default` a `packages/app/src-tauri/capabilities/default.json`.
- [x] 1.3 Añadir la dependencia frontend correspondiente (`@tauri-apps/plugin-notification` sólo si se dispara desde JS; si las notificaciones se emiten desde Rust, no hace falta) y verificar `pnpm install`.

## 2. Backend — lectura de límites (usage-limits)

- [x] 2.1 Crear módulo `src-tauri/src/limits/mod.rs`. Definir structs serde: `LimitsSnapshot { session: Option<Window>, weekly: Option<Window>, weekly_by_model: Vec<Window>, fetched_at: String, status: LimitsStatus }`, `Window { label: Option<String>, utilization: f64, resets_at: String }`, y `LimitsStatus { Ok, Unavailable { reason } }` con motivos `not_signed_in | keychain_denied | expired | network | http | parse`.
- [x] 2.2 Implementar `read_keychain_token()`: ejecutar `security find-generic-password -w -s "Claude Code-credentials"`, parsear el JSON, extraer `claudeAiOauth.accessToken` y `expiresAt`. Mapear ausencia/parseo/denegación a los motivos `Unavailable` correspondientes. Si `expiresAt < now` → `Unavailable{expired}` sin refrescar.
- [x] 2.3 Implementar `fetch_usage(token)`: `GET https://api.anthropic.com/api/oauth/usage` con headers `Authorization: Bearer` y `anthropic-beta: oauth-2025-04-20`; parseo serde tolerante (`#[serde(default)]`, campos opcionales). Derivar las ventanas del array `limits[]`: `kind=="session"`→`session`, `kind=="weekly_all"`→`weekly`, `kind=="weekly_scoped"` con `scope.model`→un `Window` en `weekly_by_model` etiquetado con `scope.model.display_name`. Fallback a `five_hour`/`seven_day` (y `seven_day_opus`/`seven_day_sonnet`) si `limits[]` está ausente. Mapear error de red → `network`, status no 2xx → `http`, fallo de parseo → `parse`. Respetar `Retry-After` en 429.
- [x] 2.4 Comando `#[tauri::command] query_limits(state) -> Result<LimitsSnapshot, String>` que combina 2.2 + 2.3.
- [x] 2.5 Registrar `query_limits` en el `invoke_handler` de `lib.rs`.

## 3. Backend — alertas de umbral

- [x] 3.1 Definir `THRESHOLDS: [u8;3] = [50,70,80]` y estado por ventana `ThresholdState { last_resets_at: String, highest_fired: u8 }`, con una clave en `meta` por ventana: `alert_session`, `alert_weekly` y `alert_weekly_scoped:<model>` (una por modelo). Persistir para sobrevivir reinicios.
- [x] 3.2 Implementar `evaluate_alerts(snapshot, &mut state)`: iterar sesión, semana y cada semanal por modelo; por ventana, si `resets_at` cambió → `highest_fired = 0`; calcular el mayor umbral cruzado por `utilization`; si supera `highest_fired`, devolver la notificación a emitir y actualizar `highest_fired`.
- [x] 3.3 Emitir notificación de macOS (título "TokenWatch", cuerpo "Sesión 5h: 70 % usado" / "Semana: 80 % usado" / "Semana Opus: 70 % usado") vía el plugin de notification cuando `evaluate_alerts` lo indique y `alerts_muted == false`.
- [x] 3.4 Ajuste `alerts_muted` persistido en `meta`; comandos `get_alerts_muted` / `set_alerts_muted(bool)`.

## 4. Backend — poll de 5 min + consumo por proyecto del día

- [x] 4.1 `LIMITS_POLL_SECS = 300`; añadir `spawn_limits_polling_task(app_handle)` (tarea `tokio` independiente del poll de ingesta) que consulta límites, corre `evaluate_alerts` + notifica, persiste estado y emite `limits-updated` con el `LimitsSnapshot`. Lanzarla en `setup()`. Refresco inmediato una vez al arrancar.
- [x] 4.2 Comando `query_today_by_project(state) -> Result<TodayByProject, String>`: `SELECT project_name, SUM(total_tokens) FROM usage_events WHERE timestamp >= <inicio de hoy local en UTC> GROUP BY project_name ORDER BY 2 DESC`; devolver `{ rows: [{project, tokens, pct}], total_tokens }` con `pct = tokens/total*100`. Registrar en `invoke_handler`.

## 5. Backend — tray + popover (menubar-popover)

- [x] 5.1 En `setup()` de `lib.rs`: `#[cfg(target_os="macos")] app.set_activation_policy(ActivationPolicy::Accessory)`.
- [x] 5.2 Construir `TrayIconBuilder` con icono y menú (`Abrir`, `Salir`); manejar `TrayIconEvent::Click` para toggle del popover.
- [x] 5.3 En el toggle: reposicionar la ventana `main` bajo el icono del tray (usar la geometría del evento) y `show() + set_focus()`; si ya visible, `hide()`.
- [x] 5.4 Suscribir `WindowEvent::Focused(false)` de la ventana `main` → `hide()`.
- [x] 5.5 Actualizar `tauri.conf.json`: ventana `main` con `visible:false`, `decorations:false`, `skipTaskbar:true`, tamaño del popover; quitar el TODO(menubar) de `lib.rs`.

## 6. Frontend — tipos y hooks

- [x] 6.1 Añadir tipos en `src/features/usage/types.ts` (o nuevo `features/limits/types.ts`): `LimitsSnapshot`, `Window`, `LimitsStatus`, `TodayByProject`, `ProjectUsageRow`.
- [x] 6.2 Hook `useLimits`: invoca `query_limits`, escucha `limits-updated`, expone snapshot/loading/error y `refresh()`.
- [x] 6.3 Hook/consulta `useTodayByProject`: invoca `query_today_by_project`, escucha `usage-updated`, expone filas + total.

## 7. Frontend — UI del popover

- [x] 7.1 Componente `LimitGauge` (barra + %) reutilizable para sesión y semana, con formateo de tiempo hasta `resets_at` ("resetea en 2h 15m").
- [x] 7.2 Componente `LimitsSection`: `LimitGauge` de sesión 5h y semana; sub-sección de semanales por modelo (un `LimitGauge` compacto por cada entrada de `weekly_by_model`, etiquetado con el modelo), omitida si la lista está vacía; estado explícito cuando `status != Ok` (mensaje "Abre Claude Code para actualizar" en `expired`/`not_signed_in`).
- [x] 7.3 Componente `TodayByProjectList`: lista ordenada de proyectos con tokens + % del total y fila de total del día; estado vacío si no hay consumo.
- [x] 7.4 Ensamblar el popover en `App.tsx` (o nuevo `Popover.tsx`): `LimitsSection` + `TodayByProjectList`; disparar refresco de ambos al montarse/mostrarse. Mantener/enlazar el acceso al gráfico existente si aplica.
- [x] 7.5 Toggle de "silenciar alertas" en la UI, enlazado a `get/set_alerts_muted`.

## 8. Verificación

- [x] 8.1 `cargo fmt`, `cargo clippy` y `cargo test` en `src-tauri` pasan.
- [x] 8.2 `pnpm typecheck && pnpm lint && pnpm test:run` pasan.
- [ ] 8.3 Prueba manual en macOS: icono en barra de menú (sin Dock); click abre popover; muestra % de sesión y semana coincidiendo con `/usage`; desglose por proyecto del día suma el total; el popover se oculta al perder foco; forzar cruce de umbral genera notificación una sola vez; silenciar la suprime.
- [ ] 8.4 Prueba de fallo: sin Claude Code / token expirado → el popover muestra estado no disponible sin romper el desglose local.
