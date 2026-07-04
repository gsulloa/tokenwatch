## Why

Hoy la app abre una ventana normal con un gráfico, pero no cumple su razón de ser: vivir en la barra de menú y decirte de un vistazo cuánto llevas consumido de tus límites de Claude. Además, el dato que de verdad importa para no quedarte cortado —el porcentaje de la ventana de 5h (sesión) y de la semana— no está en los logs JSONL: sólo lo expone Claude en `/usage`. Necesitamos leer ese dato real y avisarte antes de toparte con el límite.

## What Changes

- **Tray + popover (base del menu-bar).** Icono persistente en la barra de menú (`TrayIconBuilder`), `ActivationPolicy::Accessory` (sin Dock), y una ventana popover sin bordes que se muestra al hacer click en el icono y se oculta al perder el foco. Cubre el `TODO(menubar)` de `lib.rs`. **BREAKING**: la ventana `main` actual (420×560 visible al arrancar) pasa a ser el popover oculto por defecto.
- **Lectura de límites reales de Claude.** Nuevo módulo que lee el token OAuth desde el Keychain de macOS (`Claude Code-credentials`) y consulta `GET https://api.anthropic.com/api/oauth/usage`, obteniendo la utilización de la **sesión de 5h** (`five_hour.utilization`), de la **semana** (`seven_day.utilization`) y de los **límites semanales por modelo** (entradas `weekly_scoped` del array `limits[]`, con el modelo en `scope.model`), cada una con su `resets_at`. Comando Tauri `query_limits` + poll en segundo plano cada **5 min** que emite el evento `limits-updated`.
- **Consumo por proyecto del día (local).** Comando `query_today_by_project` que agrega `usage_events` del día actual y devuelve, por proyecto, los tokens y su **% del total del día**, más el total global. Es informativo (no dispara alertas).
- **Alertas de umbral.** Al cruzar hacia arriba **50 / 70 / 80 %** de la utilización de sesión, semana y semana-por-modelo, notificación nativa de macOS. Una notificación por umbral y por ventana; el estado se reinicia cuando cambia el `resets_at` (nueva ventana). Toggle para silenciar.
- **UI del popover.** Medidores de sesión (5h) y semana con % y tiempo hasta el reset, medidores compactos por modelo para los semanales por modelo presentes, y la lista de proyectos del día con tokens y %. Escucha `limits-updated` y `usage-updated`.
- **Dependencia nueva.** `tauri-plugin-notification` + permiso en capabilities.

### Decisión clave (reconciliación con la petición original)
El modelo de límites de Claude tiene **sólo dos ventanas: sesión de 5h y semana** (más variantes por-modelo). **No existe un límite "diario".** Como se pidió "usar la definición de Claude", las **alertas 50/70/80 aplican a sesión y semana**. El consumo "por proyecto del día" se mantiene como vista informativa local, porque Claude no expone ni límite diario ni desglose por proyecto.

## Capabilities

### New Capabilities
- `usage-limits`: Lectura de la utilización real de límites de Claude (sesión 5h y semana) desde el endpoint OAuth `/api/oauth/usage`, autenticación vía Keychain, polling cada 5 min, y alertas de umbral (50/70/80 %) con notificaciones de macOS sin spam.
- `menubar-popover`: App de barra de menú (tray icon + `ActivationPolicy::Accessory`), popover sin bordes con toggle desde el tray y ocultado al perder foco, y su contenido: medidores de sesión/semana y desglose de consumo por proyecto del día (tokens y % del total).

### Modified Capabilities
<!-- Ninguna: las requirements de usage-ingestion y usage-charts no cambian. El nuevo query por-proyecto-del-día es una lectura sobre datos existentes y vive en menubar-popover. -->

## Impact

- **Rust (`src-tauri/src`)**: nuevo módulo `limits/` (cliente HTTP + Keychain + poll + alertas), nuevo `tray`/popover en `lib.rs` (cubre `TODO(menubar)`), comandos `query_limits` y `query_today_by_project` en `usage/mod.rs`, registro en `invoke_handler`.
- **Dependencias**: `tauri-plugin-notification` (Cargo.toml + `capabilities/default.json`), cliente HTTP (`reqwest` o `ureq`) para el endpoint OAuth. `tauri.conf.json`: la ventana pasa a `visible:false`, `decorations:false`, `skipTaskbar`.
- **React (`src/`)**: nuevos componentes de popover (medidores sesión/semana, lista por proyecto), hook `useLimits` (invoca `query_limits`, escucha `limits-updated`), tipos espejo, y `query_today_by_project` en el hook de uso.
- **Externo**: llamadas de red a `api.anthropic.com` con el token del usuario; prompt único de Keychain al leer `Claude Code-credentials`. El coste sigue siendo estimado (sin cambios).
- **No incluye**: refresh/rotación del token OAuth (si el token está expirado y Claude Code lleva horas cerrado, se muestra estado "usage no disponible"); alertas por proyecto; pantalla de Settings completa.
