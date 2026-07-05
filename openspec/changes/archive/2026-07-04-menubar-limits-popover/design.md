## Context

La app es un scaffold Tauri 2 + React. Ya existe una tubería sólida: ingesta nativa de JSONL → SQLite (`usage_events`), poll cada 30s que emite `usage-updated`, tabla de precios embebida, y comandos `refresh_usage` / `query_series` / `usage_meta`. Lo que falta es (a) que la app viva en la barra de menú con un popover, y (b) el dato de límites reales de Claude, que **no está en los JSONL**.

Investigación confirmada sobre `/usage`:
- La CLI de Claude Code obtiene los límites de `GET https://api.anthropic.com/api/oauth/usage`.
- Auth: token OAuth en el **Keychain de macOS**, generic-password con `service = "Claude Code-credentials"`; el valor es JSON con `.claudeAiOauth.accessToken`, `.refreshToken`, `.expiresAt` (epoch ms, dura ~pocas horas), `.subscriptionType`, `.scopes`.
- Headers requeridos: `Authorization: Bearer <token>` y `anthropic-beta: oauth-2025-04-20`.
- Respuesta (verificada contra una cuenta real): `five_hour.utilization` (%), `five_hour.resets_at` (ISO); `seven_day.utilization` (%), `seven_day.resets_at`; opcionales `seven_day_opus`/`seven_day_sonnet`; y un array normalizado `limits[]` con `{kind: session|weekly_all|weekly_scoped, group, percent, severity, resets_at, scope:{model}, is_active}`. `limit_dollars`/`used_dollars` son null en planes de suscripción → la unidad nativa es el **porcentaje**.

Constraint central: **Claude sólo define ventana de 5h (sesión) y de 7 días (semana). No hay límite diario.** Por eso las alertas van sobre sesión y semana, y el desglose "por proyecto del día" es informativo (viene de SQLite, no del endpoint).

## Goals / Non-Goals

**Goals:**
- Icono persistente en la barra de menú que, al hacer click, muestra/oculta un popover sin bordes; el popover se oculta al perder foco. `ActivationPolicy::Accessory` (sin Dock).
- Popover que muestra: utilización de sesión (5h) y semana con % y tiempo hasta reset; y lista de proyectos del día con tokens y % del total.
- Poll de `/api/oauth/usage` cada 5 min; refresco inmediato al abrir el popover.
- Notificación nativa de macOS al cruzar 50/70/80 % de sesión y/o semana, sin spam.

**Non-Goals:**
- Refresh/rotación del token OAuth (si expiró y Claude Code lleva horas cerrado → estado "usage no disponible"). No escribimos en el Keychain de Claude Code.
- Alertas o presupuestos por proyecto; pantalla de Settings completa (rutas/fuentes). Codex.
- Cambiar la ingesta JSONL o `query_series` existentes.

## Decisions

### D1: Fuente de los límites = endpoint OAuth `/api/oauth/usage` (no cálculo por tokens)
Los % de sesión/semana de Claude no son derivables de tokens (Anthropic no publica los cupos). El endpoint devuelve `utilization` directo, idéntico a lo que ve el usuario en `/usage`.
- *Alternativa descartada*: inferir % con presupuestos de tokens configurables → no coincidiría con `/usage`, que es justo lo que el usuario pidió replicar.
- *Alternativa descartada*: leer los headers `anthropic-ratelimit-unified-*` de respuestas normales → sólo disponibles tras una llamada de inferencia; el endpoint dedicado es determinista.

### D2: Auth vía Keychain, lectura por ciclo, sin refresh
El backend lee el token con `security find-generic-password -w -s "Claude Code-credentials"` (subproceso) en cada ciclo de poll. Como Claude Code mantiene el token fresco y dura horas, un poll de 5 min casi siempre encuentra un token válido. Si `expiresAt < now`, no refrescamos (evita clobber/rotación del refresh token de Claude): se emite estado `Unavailable{reason:"expired"}` y la UI sugiere abrir Claude Code.
- *Alternativa descartada*: crate de keychain (`security-framework`) → el subproceso `security` es más simple y equivale al prompt estándar de macOS.
- *Alternativa descartada*: implementar el refresh OAuth nosotros → riesgo de rotar/invalidar el refresh token compartido con Claude Code. Se deja como mejora futura.

### D3: Cliente HTTP mínimo
Usar `reqwest` con `rustls` (o `ureq` bloqueante) para el GET. Elegimos **`reqwest`** (ya hay `tokio` en el árbol; encaja con el poll async).

### D4: Ventanas = tal cual las reporta Claude, parseadas desde `limits[]`
Usamos el array normalizado `limits[]` como fuente primaria porque unifica todas las ventanas con la misma forma (`kind`, `percent`, `resets_at`, `scope.model`, `severity`):
- `kind == "session"` → **sesión (5h)**.
- `kind == "weekly_all"` → **semana** (total).
- `kind == "weekly_scoped"` con `scope.model` → **semana por modelo** (0..N; p.ej. "Fable", "Opus"), etiquetada con `scope.model.display_name`.

Si `limits[]` viniera ausente, se cae a los campos top-level `five_hour`/`seven_day` (y `seven_day_opus`/`seven_day_sonnet` si están). Mostramos `percent`/`utilization` y `resets_at` directos: nada de bloques anclados ni rolling propios (usamos la definición de Claude). El `LimitsSnapshot` expone `session: Option<Window>`, `weekly: Option<Window>` y `weekly_by_model: Vec<Window>`, donde `Window { label: Option<String>, utilization: f64, resets_at: String }`.

### D5: Popover = reutilizar la ventana `main`, no crear una nueva
Cambiamos la ventana `main` a `visible:false`, `decorations:false`, `skipTaskbar:true`, `alwaysOnThruogh` no. En `setup()`: `set_activation_policy(Accessory)`, construir `TrayIconBuilder` con icono + menú (Abrir / Salir), y en el evento click reposicionar la ventana bajo el icono del tray y `show()+set_focus()`. Suscribir `WindowEvent::Focused(false)` → `hide()`.
- *Alternativa descartada*: crate `tauri-plugin-positioner` / popover libs → el reposicionado manual bajo el tray con la geometría del `TrayIconEvent` basta para v1.

### D6: Modelo de alertas — máquina de umbrales por ventana
Estado persistido en la tabla `meta`, con **una clave por ventana**: `session`, `weekly` y `weekly_scoped:<model>` (una por modelo presente): `{last_resets_at, highest_threshold_fired}`. La misma máquina aplica a todas las ventanas de forma uniforme, así que los semanales por modelo alertan igual que sesión y semana. En cada poll:
1. Si `resets_at` cambió respecto a `last_resets_at` → nueva ventana: `highest_threshold_fired = 0`.
2. Determinar el mayor umbral de `[50,70,80]` que `utilization` supera.
3. Si ese umbral > `highest_threshold_fired` → emitir **una** notificación ("Sesión 5h: 70% usado") y actualizar `highest_threshold_fired`.
Esto garantiza: una notificación por umbral por ventana, sin re-disparo al oscilar, y reset limpio al renovarse la ventana. Un toggle `alerts_muted` (persistido) corta todas las notificaciones.
- *Umbrales*: constantes `[50,70,80]` con forma de lista para volverlos configurables luego sin refactor.

### D7: Consumo por proyecto del día — nuevo comando de sólo lectura
`query_today_by_project` corre `SELECT project_name, SUM(total_tokens) ... WHERE timestamp >= <inicio de hoy, hora local> GROUP BY project_name ORDER BY 2 DESC`. Devuelve filas `{project, tokens, pct}` (pct = tokens/total_del_día·100) + `total_tokens`. "Hoy" en hora local del usuario. Reutiliza índices existentes (`idx_ue_timestamp`, `idx_ue_project`).

### D8: Eventos y polling separados
- El poll de 30s de ingesta JSONL y su `usage-updated` **no cambian**.
- Nuevo poll de límites cada 5 min (`LIMITS_POLL_SECS = 300`) en su propia tarea `tokio`, que emite `limits-updated` con el `LimitsSnapshot`. Al mostrar el popover, el frontend invoca `query_limits` para refresco inmediato (no espera 5 min).

## Risks / Trade-offs

- [Token expirado tras horas sin Claude Code] → estado `Unavailable` explícito en el popover ("Abre Claude Code para actualizar"); no rompe el resto de la UI (el desglose local sigue funcionando).
- [Prompt de Keychain molesto] → sucede una vez; documentar que hay que elegir "Permitir siempre". Si el usuario deniega, estado `Unavailable{reason:"keychain_denied"}`.
- [Cambio de esquema del endpoint no documentado] → parsear con serde tolerante (`#[serde(default)]`, campos opcionales); si falla el parseo, `Unavailable` + log, sin crashear el poll.
- [Rate limiting del propio endpoint] → 5 min es benigno; respetar `Retry-After` si llega 429 (backoff simple).
- [BREAKING: ventana ya no visible al arrancar] → aceptado; es el comportamiento correcto de un menu-bar app. Rollback = revertir `tauri.conf.json` + `setup()`.
- [Zona horaria de "hoy"] → usar hora local del SO para el corte diario; documentar que los `timestamp` en SQLite son UTC y se comparan con el inicio de día local convertido a UTC.

## Migration Plan

1. Añadir `tauri-plugin-notification` + `reqwest` a Cargo.toml; permiso notification en `capabilities/default.json`.
2. Implementar `limits/` (keychain + cliente + parseo + alertas) y comandos; registrar en `invoke_handler`.
3. Convertir la ventana a popover + tray en `lib.rs` y `tauri.conf.json`.
4. Frontend: `useLimits`, componentes de popover, `query_today_by_project`.
5. Verificación: `pnpm typecheck && pnpm lint && pnpm test:run` y `cargo fmt/clippy/test`.
- **Rollback**: revertir cambios de ventana/tray restaura la ventana normal; los comandos nuevos son aditivos y no afectan la ingesta existente.

## Open Questions

- Semanales por modelo: **resuelto** — se muestran (medidor compacto por modelo) y alertan como cualquier otra ventana (D4 + D6). Sólo aparecen los que el endpoint reporte (`weekly_scoped`); si no hay ninguno, la sección por modelo se omite.
- ¿Persistir el `highest_threshold_fired` en disco o sólo en memoria? **Resuelto**: persistir en `meta` (una clave por ventana) para que reiniciar la app no re-dispare notificaciones dentro de la misma ventana.
