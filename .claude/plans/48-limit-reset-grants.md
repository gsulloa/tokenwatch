# Plan — #48 Avisar cuando haya reinicios gratuitos de límites disponibles

Rama: `feat/48-limit-reset-grants` (desde `origin/dev`). Base de la PR: `dev`.

## 1. Contexto

**Qué pide la issue.** Anthropic entrega reinicios gratuitos de límites ("grants")
que se reclaman a mano en claude.ai → Configuración → Uso. El mismo endpoint que ya
usa TokenWatch los expone en el bloque `cedar_ember` si se piden con
`?cedar_ember=1&skip_spend=1` y el `User-Agent` es el de Claude Code CLI. Hay que:
(1) leer `cedar_ember` en el poll existente, sin peticiones nuevas ni intervalos más cortos;
(2) notificar por macOS cuando aparece un grant disponible no notificado y cuando uno sin
usar está por vencer; (3) mostrar en el popover cuántos hay y cuándo vence el más próximo;
(4) si el bloque viene vacío, `null` o no elegible, no mostrar nada ni dar errores.

**Qué existe hoy.**
- `packages/app/src-tauri/src/limits/mod.rs`: `fetch_usage(token)` hace
  `GET https://api.anthropic.com/api/oauth/usage` con `Authorization` y
  `anthropic-beta: oauth-2025-04-20`, **sin `User-Agent`** (reqwest no pone uno por
  defecto). Deserializa en `ApiResponse` (`limits`, `five_hour`, `seven_day`,
  `seven_day_opus`, `seven_day_sonnet`) y `parse_api_response(api, now)` arma
  `LimitsSnapshot`. Poll cada `LIMITS_POLL_SECS = 300` en `spawn_limits_polling_task` →
  `run_limits_and_emit`, que evalúa alertas bajo el lock de `conn`, respeta
  `alerts_muted` (meta) y emite `limits-updated`.
- **`skip_spend=1` no afecta nada en uso**: `ApiResponse` no modela ningún campo
  `spend` (ni hay referencias a `spend` en el módulo); los campos usados (`limits[]`,
  `five_hour`, `seven_day`, `seven_day_opus`, `seven_day_sonnet`) siguen presentes según
  la prueba en vivo. Serde ignora campos desconocidos.
- Notificaciones: `tauri_plugin_notification`, título `"TokenWatch"`, cuerpo en español
  (`"Semana: 70 % usado"`). Estado de alertas persistido en la tabla `meta`
  (`db::meta_get/meta_set`) como JSON. Mientras `alerts_muted` está activo, el estado de
  umbrales **igual avanza** (no se acumulan alertas para cuando se des-silencie).
- Frontend: `src/features/limits/{types.ts,useLimits.ts,LimitsSection.tsx}`;
  `LimitsSection` se renderiza dentro de `LiveStatusPanel`, que usan tanto el popover
  (`app/Popover.tsx`) como el dashboard (`app/App.tsx`), así que la fila nueva aparecerá
  en ambos (paridad existente; está bien).
- Specs vivas: `openspec/specs/usage-limits/spec.md` (requisito "Consulta de utilización
  de límites de Claude" fija los headers; hay que actualizarlo) y
  `openspec/specs/menubar-popover/spec.md` (requisito "Medidores de sesión, semana y semana
  por modelo"). La issue no las contradice; las amplía.

**Decisiones tomadas (del usuario, no reabrir):**
- Una sola petición: la URL existente pasa a
  `https://api.anthropic.com/api/oauth/usage?cedar_ember=1&skip_spend=1`. Mismo intervalo de 300 s.
- Se agrega el header `User-Agent: claude-cli/<versión> (external, cli)`. `x-app` no hace falta.
- Versión: mayor semver válido (`X.Y.Z` numérico) entre los **nombres** de las entradas de
  `~/.local/share/claude/versions/`; fallback constante `"2.1.328"`. Solo se leen nombres
  (no se abre ningún archivo; se aceptan entradas de cualquier tipo — archivo, dir o
  symlink — porque el instalador nativo puede guardar binarios ahí). Se calcula **una vez
  por proceso** (`OnceLock`) en la primera petición; no se re-escanea en cada poll.
- Parseo tolerante: `cedar_ember` ausente, `null`, mal tipado o `eligible != true` →
  lista de grants vacía, sin error, sin UI. El bloque se deserializa aparte desde
  `serde_json::Value` para que un error ahí **nunca** invalide el resto de la respuesta.
- Grant "disponible" = `paused != true && resets_left > 0 && (ends_at ausente || ends_at > now)`.
  Grants sin `id` se descartan.
- Notificaciones (título `"TokenWatch"`, respetan `alerts_muted`):
  a) grant disponible cuyo id no está en el set de notificados → notificar. En la primera
     ejecución (meta vacío) se notifican todos los disponibles existentes.
  b) grant disponible con `ends_at` dentro de ≤ 3 días → notificar una sola vez por grant.
- Popover: fila compacta "N reinicios disponibles · vence el <fecha>" dentro de la
  sección de límites; si no hay disponibles, no se renderiza.
- `LimitsSnapshot` gana `resetGrants` (camelCase) con el resumen de grants disponibles.
- Fuera de alcance: reclamar (ningún POST); detección de caídas de uso.

**Defaults elegidos por el planner (anotados):**
- **Sin click en la fila.** No existe un mecanismo verificado para abrir URLs externas
  en el sistema: no hay `tauri-plugin-opener`/`shell` en `Cargo.toml` ni en
  `capabilities/default.json`; el único precedente es `window.open` en
  `components/Markdown/Markdown.tsx`, sin evidencia de que abra el navegador del sistema
  desde el WKWebView de Tauri. Por la regla de "no agregar capabilities sin justificar",
  la fila es informativa y la notificación indica dónde reclamar. Agregar el opener queda
  en seguimiento.
- **N = suma de `resetsLeft`** de los grants disponibles (son "reinicios", no "grants").
  Singular/plural: `1 reinicio disponible` / `N reinicios disponibles`.
- **"vence el <fecha>"** (con "el", igual que la notificación y que "cierra el …" de
  `LimitGauge`). Si el grant más próximo no tiene `endsAt`, se omite `· vence el …`.
- **Grant nuevo que además ya vence en ≤ 3 días**: se envía **una** notificación (la de
  "disponible", que ya incluye la fecha) y se marca el id en ambos sets, para no notificar
  dos veces en el mismo ciclo.
- **Silenciado**: igual que los umbrales existentes — el estado de notificados avanza y
  no se envía nada; al des-silenciar no se repiten avisos viejos. La fila del popover sigue visible.
- **Persistencia**: meta key `reset_grants_notified`, JSON
  `{"available":[ids…],"expiring":[ids…]}`. Sin poda por presencia (un grant que
  desaparece y reaparece no se re-notifica); cada lista se acota a los **200** ids más
  recientes (se descartan los más viejos).
- **Fecha en la notificación (Rust)**: chrono sin locale → formateo manual
  `"<día> <mes abreviado>"` en hora local, p.ej. `"30 nov"`, con meses
  `ene feb mar abr may jun jul ago sept oct nov dic` (coincide con
  `toLocaleDateString("es", {day:"numeric", month:"short"})` del frontend).
- **Sin migración de DB**: se usa la tabla `meta` existente.

## 2. Archivos a tocar

| Ruta | Acción | Qué cambia |
|---|---|---|
| `packages/app/src-tauri/src/limits/cli_version.rs` | crear | Detección de versión de Claude Code CLI y `User-Agent` cacheado. |
| `packages/app/src-tauri/src/limits/reset_grants.rs` | crear | Structs API de `cedar_ember`, `ResetGrant` (IPC), selección de disponibles, máquina de notificaciones, formateo de fecha. |
| `packages/app/src-tauri/src/limits/mod.rs` | modificar | `mod` de los submódulos; URL con query; header `User-Agent`; `ApiResponse.cedar_ember`; `LimitsSnapshot.reset_grants`; evaluación y envío de notificaciones en `run_limits_and_emit`; tests. |
| `packages/app/src-tauri/src/ingest/mod.rs` | modificar | `fn home_dir()` → `pub(crate) fn home_dir()` (reutilizar). |
| `packages/app/src-tauri/src/menubar.rs` | modificar | Agregar `reset_grants: vec![]` a los 2 literales `LimitsSnapshot` de tests (líneas ~178 y ~256). |
| `packages/app/src-tauri/src/budgets/mod.rs` | modificar | Agregar `reset_grants: vec![]` a los 2 literales `LimitsSnapshot` de tests (líneas ~616 y ~1017). |
| `packages/app/src/features/limits/types.ts` | modificar | Tipo `ResetGrantSummary`; campo `resetGrants` en `LimitsSnapshot`. |
| `packages/app/src/features/limits/useLimits.ts` | modificar | Normalizar `resetGrants` del snapshot crudo. |
| `packages/app/src/features/limits/resetGrants.ts` | crear | Funciones puras `summarizeResetGrants` y `formatResetGrantsLabel`. |
| `packages/app/src/features/limits/resetGrants.test.ts` | crear | Tests de las funciones puras. |
| `packages/app/src/features/limits/ResetGrantsRow.tsx` | crear | Fila compacta del popover. |
| `packages/app/src/features/limits/LimitsSection.tsx` | modificar | Renderizar `ResetGrantsRow` en la rama `ok`. |
| `packages/app/src/features/limits/LimitsSection.test.tsx` | modificar | `resetGrants: []` en `makeSnapshot` + tests de la fila. |
| `openspec/specs/usage-limits/spec.md` | modificar | Actualizar requisito de consulta (URL/UA) + requisitos nuevos de grants y sus notificaciones. |
| `openspec/specs/menubar-popover/spec.md` | modificar | Requisito nuevo de la fila de reinicios. |

No se toca: `Cargo.toml` (no hay dependencias nuevas; `tempfile` ya es dev-dependency),
`capabilities/default.json`, `tauri.conf.json` (CSP), `lib.rs` (no hay comandos nuevos),
`db/`, `CHANGELOG.md`, versiones.

## 3. Pasos ordenados

### Paso 1 — `ingest/mod.rs`: exponer `home_dir`
Cambiar `fn home_dir() -> Option<std::path::PathBuf>` (línea ~615) a
`pub(crate) fn home_dir() -> Option<std::path::PathBuf>`. Sin otros cambios.

### Paso 2 — crear `limits/cli_version.rs`

```rust
//! Detects the installed Claude Code CLI version to build the User-Agent sent to
//! the usage endpoint. Only directory-entry NAMES are read; no file is opened.

use std::path::Path;
use std::sync::OnceLock;

pub(crate) const FALLBACK_CLI_VERSION: &str = "2.1.328";

/// Highest valid `X.Y.Z` (all numeric) among the entry names in `dir`.
/// Entries of any type (file, dir, symlink) count; names that are not exactly
/// three dot-separated u64 components are ignored. `None` if the dir is missing,
/// unreadable or has no valid name.
pub(crate) fn detect_cli_version(dir: &Path) -> Option<String>

/// Parse "X.Y.Z" into (u64,u64,u64). Rejects "1.2", "1.2.3.4", "1.2.3-beta", "v1.2.3", "".
fn parse_semver(name: &str) -> Option<(u64, u64, u64)>

/// `"claude-cli/<ver> (external, cli)"`
pub(crate) fn user_agent_for(version: &str) -> String

/// Cached once per process: `~/.local/share/claude/versions` → detect → fallback.
pub(crate) fn claude_cli_user_agent() -> &'static str
```

- `detect_cli_version`: `std::fs::read_dir(dir).ok()?`, iterar entradas
  (`filter_map(Result::ok)`), `entry.file_name().to_str()` → `parse_semver`, quedarse con
  el máximo por tupla y devolver el nombre original. No llamar `metadata()` ni abrir nada.
- `claude_cli_user_agent`: `static UA: OnceLock<String>`;
  `UA.get_or_init(|| { let v = crate::ingest::home_dir().map(|h| h.join(".local/share/claude/versions")).and_then(|d| detect_cli_version(&d)).unwrap_or_else(|| FALLBACK_CLI_VERSION.to_owned()); tracing::debug!("limits: claude-cli UA version {v}"); user_agent_for(&v) })`.
  Devolver `UA.get_or_init(...).as_str()`.

### Paso 3 — crear `limits/reset_grants.rs`

**Structs de la API (privados al crate, tolerantes):**

```rust
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct ApiCedarEmber {
    pub eligible: Option<bool>,
    pub ineligible_reason: Option<String>,
    pub grants: Option<Vec<serde_json::Value>>,   // per-row from_value, malformed rows skipped
    pub next_grant_id: Option<String>,
    pub cooldown_until: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct ApiGrant {
    pub id: Option<String>,
    pub label: Option<String>,
    pub resets_total: Option<i64>,
    pub resets_left: Option<i64>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub clears: Option<Vec<String>>,
    pub paused: Option<bool>,
    pub usable_now: Option<bool>,
    #[serde(rename = "type")]
    pub kind: Option<String>,            // "session" | "weekly" | "full" | otro
    pub unavailable_reason: Option<String>,
    pub gives_back_weekly: Option<bool>,
    pub claimable: Option<bool>,
}
```
(Campos que no se usan en la lógica se modelan igual para documentar el contrato; si
clippy marca `dead_code`, anotar el struct con `#[allow(dead_code)]` y un comentario
"fields kept to document the API contract".)

**Tipo IPC (público, va en `LimitsSnapshot`):**

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetGrant {
    pub id: String,
    pub label: String,
    pub resets_left: u32,
    /// RFC 3339 tal como llegó (solo si parsea); None si falta o no parsea.
    pub ends_at: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub usable_now: bool,
}
```
TS espejo (Paso 7): `{ id: string; label: string; resetsLeft: number; endsAt: string | null; type: string | null; usableNow: boolean }`.

**Funciones:**

```rust
pub(crate) const DEFAULT_GRANT_LABEL: &str = "Reinicio de límites";
pub(crate) const EXPIRING_WINDOW_DAYS: i64 = 3;
pub(crate) const NOTIFIED_CAP: usize = 200;
pub(crate) const GRANTS_META_KEY: &str = "reset_grants_notified";

/// Parse the raw `cedar_ember` value into the available grants, sorted by
/// `ends_at` ascending (None last). Returns empty on: None, JSON null, wrong
/// type, `eligible != Some(true)`, `grants` null/missing.
pub(crate) fn available_grants(raw: Option<serde_json::Value>, now: DateTime<Utc>) -> Vec<ResetGrant>
```
Lógica: `serde_json::from_value::<ApiCedarEmber>(raw?)` → si `Err` devolver `vec![]`
(log `tracing::debug!` sin contenido); si `eligible != Some(true)` → `vec![]`. Para cada
`grants[i]`: `from_value::<ApiGrant>` (descartar si falla); descartar si `id` es `None`
o vacío; `resets_left = resets_left.unwrap_or(0)`; descartar si `paused == Some(true)`
o `resets_left <= 0`; parsear `ends_at` con `DateTime::parse_from_rfc3339` → si parsea y
`<= now` descartar; si no parsea, `ends_at = None`. `label` = `label` no vacío o
`DEFAULT_GRANT_LABEL`. `resets_left` a `u32` con `u32::try_from(..).unwrap_or(u32::MAX)`.
`usable_now = usable_now.unwrap_or(false)`. Ordenar por `ends_at` (parseado) ascendente,
`None` al final.

```rust
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct GrantNotifyState {
    pub available: Vec<String>,
    pub expiring: Vec<String>,
}

/// Pure state machine. Mutates `state` and returns the notification bodies to send
/// (in grant order). Does not know about mute.
pub(crate) fn next_grant_notifications(
    state: &mut GrantNotifyState,
    grants: &[ResetGrant],
    now: DateTime<Utc>,
) -> Vec<String>
```
Por cada grant (ya son disponibles):
- `expiring_now` = `ends_at` parsea y `0 < ends_at - now <= Duration::days(EXPIRING_WINDOW_DAYS)`.
- Si `id ∉ state.available`: push body **disponible**; push id a `available`; si
  `expiring_now` y `id ∉ expiring`, push id a `expiring` (sin segundo body).
- Si no, y `expiring_now` y `id ∉ state.expiring`: push body **por vencer**; push id a `expiring`.
Al final, si alguna lista supera `NOTIFIED_CAP`, `drain(..len - NOTIFIED_CAP)` (descarta
los más viejos).

**Copy exacto** (`<fecha>` = `format_short_date_es(ends_at en Local .date_naive())`):
- Disponible con fecha: `"Reinicio de límites disponible: {label} · vence el {fecha}. Reclámalo en claude.ai → Configuración → Uso"`
- Disponible sin `ends_at`: `"Reinicio de límites disponible: {label}. Reclámalo en claude.ai → Configuración → Uso"`
- Por vencer: `"Reinicio de límites por vencer: {label} · vence el {fecha}. Reclámalo en claude.ai → Configuración → Uso"`

Para que los tests no dependan de la zona horaria de la máquina, separar:
`fn body_available(label: &str, date: Option<&str>) -> String` y
`fn body_expiring(label: &str, date: &str) -> String`, y que
`next_grant_notifications` convierta con `chrono::Local`. Los tests de la máquina usan
`ends_at` a mediodía UTC (p.ej. `T12:00:00Z`) para que el día local sea estable en
cualquier zona entre UTC−11 y UTC+11, o comparan con `contains("vence el")` + label.

```rust
/// "30 nov". Months: ene feb mar abr may jun jul ago sept oct nov dic.
pub(crate) fn format_short_date_es(date: chrono::NaiveDate) -> String

/// Load state from meta, run the state machine, persist if changed, and return
/// the bodies to send — or an empty vec when `muted` (state still advances).
pub(crate) fn evaluate_grant_alerts(
    conn: &rusqlite::Connection,
    grants: &[ResetGrant],
    now: DateTime<Utc>,
    muted: bool,
) -> Vec<String>
```
`evaluate_grant_alerts`: `db::meta_get(conn, GRANTS_META_KEY)` → `serde_json::from_str`
(si falla o no existe → `GrantNotifyState::default()`); clonar para comparar; correr
`next_grant_notifications`; si cambió, `db::meta_set` con `serde_json::to_string`
(ignorar error con `let _ =` como en `evaluate_window`); devolver `if muted { vec![] } else { bodies }`.
Si `grants` está vacío, no tocar meta (retorno temprano con `vec![]`).

### Paso 4 — `limits/mod.rs`

1. Arriba, tras los `use`: `mod cli_version;` y `pub mod reset_grants;` y
   `pub use reset_grants::ResetGrant;`.
2. Constante: `const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage?cedar_ember=1&skip_spend=1";`
3. En `fetch_usage`, el builder queda:
   `.get(USAGE_URL)` + `.header("Authorization", …)` + `.header("anthropic-beta", "oauth-2025-04-20")`
   + `.header(reqwest::header::USER_AGENT, cli_version::claude_cli_user_agent())`.
   Nada más cambia (reintentos, 429, errores). El token sigue sin loguearse; **no**
   loguear el UA completo junto a nada del token.
4. `ApiResponse`: agregar `cedar_ember: Option<serde_json::Value>,`.
5. `LimitsSnapshot`: agregar al final
   ```rust
   /// Available free limit-reset grants (empty when not eligible / absent).
   #[serde(default)]
   pub reset_grants: Vec<ResetGrant>,
   ```
   y `reset_grants: vec![]` en `LimitsSnapshot::unavailable`.
6. `parse_api_response`: antes del `Ok(...)`, `let reset_grants = reset_grants::available_grants(api.cedar_ember, now);`
   (mover/tomar `api.cedar_ember` antes de que `api` se consuma parcialmente; como los
   campos se mueven uno a uno, basta con tomarlo en cualquier punto antes del final) y
   pasarlo al struct. Un `cedar_ember` inválido **no** puede devolver `Err`.
7. `run_limits_and_emit`: dentro del closure bajo `state.conn.lock()`, calcular además
   `let grant_bodies = reset_grants::evaluate_grant_alerts(&conn, &snapshot.reset_grants, Utc::now(), muted);`
   y devolverlo en la tupla (`(muted, global_alerts, group_notifications, grant_bodies)`).
   Fuera del lock, enviar cada body de `grant_bodies` (ya vienen vacíos si `muted`) con el
   mismo builder: `.title("TokenWatch").body(&text).show()`, `tracing::warn!("failed to send reset-grant notification: {e}")` en error.
8. Actualizar `ok_snapshot()` del módulo de tests con `reset_grants: vec![]`.
9. Actualizar el doc-comment del módulo: mencionar el bloque `cedar_ember` y el UA.

### Paso 5 — literales de test en otros módulos
`menubar.rs` (2 literales en `#[cfg(test)]`) y `budgets/mod.rs` (2 literales): agregar
`reset_grants: vec![],`. Correr `cargo build --all-targets` para encontrar cualquier otro.

### Paso 6 — `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test` en `packages/app/src-tauri`.

### Paso 7 — `features/limits/types.ts`
```ts
/** Free limit-reset grant available to claim in claude.ai (summary from backend). */
export interface ResetGrantSummary {
  id: string;
  label: string;
  resetsLeft: number;
  /** ISO 8601 or null when the grant has no expiry. */
  endsAt: string | null;
  /** "session" | "weekly" | "full" or other future values. */
  type: string | null;
  usableNow: boolean;
}
```
En `LimitsSnapshot` agregar `resetGrants: ResetGrantSummary[];` (requerido).

### Paso 8 — `useLimits.ts`
- `RawSnapshot`: agregar `resetGrants?: RawResetGrant[] | null; reset_grants?: RawResetGrant[] | null;`
  con `interface RawResetGrant { id?: string; label?: string; resetsLeft?: number; endsAt?: string | null; type?: string | null; usableNow?: boolean }`.
- `normalizeSnapshot`: `resetGrants: (raw.resetGrants ?? raw.reset_grants ?? []).filter((g) => typeof g.id === "string" && g.id !== "").map((g) => ({ id: g.id as string, label: g.label ?? "Reinicio de límites", resetsLeft: g.resetsLeft ?? 0, endsAt: g.endsAt ?? null, type: g.type ?? null, usableNow: g.usableNow ?? false }))`.

### Paso 9 — crear `features/limits/resetGrants.ts`
```ts
export interface ResetGrantsSummary { count: number; nearestEndsAt: string | null }

/** Sum of resetsLeft (>0 only) and the earliest valid endsAt; null when count is 0. */
export function summarizeResetGrants(grants: ResetGrantSummary[]): ResetGrantsSummary | null

/** "30 nov" via toLocaleDateString("es", { day: "numeric", month: "short" }); "" if invalid. */
export function formatGrantDate(iso: string): string

/** "1 reinicio disponible" | "N reinicios disponibles", + " · vence el <fecha>" when nearestEndsAt formats. */
export function formatResetGrantsLabel(summary: ResetGrantsSummary): string
```
`summarizeResetGrants`: ignora `resetsLeft <= 0`; `nearestEndsAt` = mínimo por
`new Date(x).getTime()` entre los `endsAt` no nulos y válidos (`!Number.isNaN`).

### Paso 10 — crear `features/limits/ResetGrantsRow.tsx`
```tsx
interface ResetGrantsRowProps { grants: ResetGrantSummary[] }
export function ResetGrantsRow({ grants }: ResetGrantsRowProps): JSX.Element | null
```
- `const summary = summarizeResetGrants(grants); if (!summary) return null;`
- Markup (estilo inline como el resto de `limits/`, siguiendo DESIGN.md: compacto, copy
  operativo, color = estado, sin morado ni sombras):
  ```tsx
  <div role="status" aria-label="Reinicios de límites"
       title="Reclámalo en claude.ai → Configuración → Uso"
       style={{ display: "flex", alignItems: "center", gap: "var(--space-xs)",
                fontSize: 12, color: "var(--text-muted)" }}>
    <span aria-hidden style={{ width: 6, height: 6, borderRadius: "var(--radius-full)",
                               background: "var(--info)", flexShrink: 0 }} />
    <span style={{ fontFamily: "var(--font-data)", fontVariantNumeric: "tabular-nums" }}>
      {formatResetGrantsLabel(summary)}
    </span>
  </div>
  ```
  Sin `onClick`, sin `cursor: pointer`, sin enlace (ver Contexto).

### Paso 11 — `LimitsSection.tsx`
En la rama no-`isUnavailable` (dentro del `div` columna con `gap: var(--space-sm)`),
agregar como **último hijo** (después del bloque "Por modelo"):
`<ResetGrantsRow grants={snapshot.resetGrants} />`. No se muestra en estados
loading / sin snapshot / unavailable.

### Paso 12 — tests TS (ver sección 4) y `pnpm typecheck && pnpm lint && pnpm test:run`.

### Paso 13 — specs de openspec (texto exacto en sección 9).

### Paso 14 — `pnpm check` desde la raíz.

## 4. Tests

### Rust — `limits/cli_version.rs` (`#[cfg(test)] mod tests`, `tempfile::tempdir()`)
- `detect_picks_highest_semver` — crea entradas `2.1.9`, `2.1.10`, `2.0.99` (mezclando
  `create_dir` y `File::create`) → `Some("2.1.10")` (comparación numérica, no léxica).
- `detect_ignores_invalid_names` — `latest`, `2.1`, `2.1.3-beta`, `v3.0.0`, `.DS_Store`,
  `1.0.0` → `Some("1.0.0")`.
- `detect_missing_dir_returns_none` — ruta inexistente dentro del tempdir → `None`.
- `detect_empty_dir_returns_none`.
- `user_agent_format` — `user_agent_for("2.1.328") == "claude-cli/2.1.328 (external, cli)"`.

### Rust — `limits/reset_grants.rs` (`#[cfg(test)]`, `now = 2026-11-07T12:00:00Z`, datos inventados)
Parseo / selección (`available_grants`):
- `cedar_none_returns_empty` — `None` → `[]`.
- `cedar_null_returns_empty` — `Some(Value::Null)` → `[]`.
- `cedar_wrong_type_returns_empty` — `Some(json!("x"))` y `Some(json!([1]))` → `[]`.
- `cedar_ineligible_returns_empty` — `{"eligible":false,"ineligible_reason":"surface","grants":[<grant válido>]}` → `[]`.
- `cedar_eligible_missing_flag_returns_empty` — sin `eligible` → `[]`.
- `cedar_grants_null_returns_empty` — `{"eligible":true,"grants":null}` → `[]`.
- `eligible_mixed_grants_selects_available` — grants: A disponible (`resets_left:1`,
  `ends_at` futuro); B `paused:true`; C `resets_left:0`; D `ends_at` pasado; E sin `id`;
  F malformado (`resets_left:"uno"`); G disponible sin `ends_at` → resultado `[A, G]`
  en ese orden, con `kind`, `usable_now`, `ends_at` correctos.
- `available_sorted_by_ends_at_none_last` — tres grants con fechas desordenadas + uno sin fecha.
- `unknown_fields_are_ignored` — grant y bloque con campos extra (`"foo":{"bar":1}`,
  `at_limit`, `weekly_resets_at`) → parsea igual.
- `missing_label_uses_default` — label ausente o `""` → `"Reinicio de límites"`.
- `unparseable_ends_at_becomes_none` — `ends_at:"mañana"` → grant disponible con `ends_at: None`.

Máquina de notificaciones (`next_grant_notifications`):
- `first_run_notifies_all_available` — estado default + 2 grants → 2 bodies "Reinicio de
  límites disponible: …", ambos ids en `available`.
- `does_not_repeat_available` — segunda llamada con el mismo estado y grants → `[]`.
- `new_grant_after_first_run_is_notified` — añadir un grant nuevo → 1 body, solo del nuevo.
- `expiring_notified_once` — grant ya en `available`, `ends_at = now + 2d` → 1 body "por
  vencer"; llamada repetida → `[]`.
- `expiring_boundary` — `now + 3d` exacto → notifica; `now + 3d + 1s` → no.
- `new_and_expiring_sends_single_notification` — estado vacío, `ends_at = now + 1d` → 1
  body (disponible), id en `available` y `expiring`; repetir → `[]`.
- `body_without_ends_at_omits_date` — body exacto `"Reinicio de límites disponible: X. Reclámalo en claude.ai → Configuración → Uso"`.
- `body_with_date_exact` — `body_available("X", Some("30 nov"))` y `body_expiring("X","30 nov")` con el copy exacto de la sección 3.
- `notified_lists_are_capped` — estado con 200 ids + 1 grant nuevo → `available.len() == 200`, el más viejo fuera, el nuevo dentro.
- `format_short_date_es_months` — `2026-01-05` → `"5 ene"`, `2026-09-30` → `"30 sept"`, `2026-11-30` → `"30 nov"`.

Con DB (`db::open_at(":memory:")`, `evaluate_grant_alerts`):
- `evaluate_persists_and_does_not_repeat` — 1ª llamada devuelve 1 body; 2ª `[]`; `meta_get(GRANTS_META_KEY)` contiene el id.
- `evaluate_muted_returns_empty_but_advances_state` — `muted=true` → `[]`; luego `muted=false` con los mismos grants → `[]`.
- `evaluate_corrupt_meta_resets_state` — `meta_set(GRANTS_META_KEY, "{not json")` → se trata como vacío y notifica.
- `evaluate_empty_grants_does_not_write_meta` — `[]` → `meta_get` sigue `None`.

### Rust — `limits/mod.rs` (`parse_api_response`, helper `parse` existente)
- `parse_without_cedar_ember_has_no_grants` — JSON existente sin el bloque → `reset_grants.is_empty()` y ventanas iguales.
- `parse_with_eligible_cedar_ember_exposes_grants` — `limits[]` + `cedar_ember` elegible con 1 grant → 1 `ResetGrant` y sesión/semana intactas.
- `parse_with_malformed_cedar_ember_keeps_limits` — `"cedar_ember": 42` → `Ok`, ventanas presentes, sin grants.
- `snapshot_serializes_reset_grants_camel_case` — `serde_json::to_value(snapshot)` contiene `resetGrants[0].resetsLeft`, `endsAt`, `type`, `usableNow`, y **no** contiene ninguna clave `token`/`authorization`.

### TS — `src/features/limits/resetGrants.test.ts` (vitest, sin Tauri)
- `summarizeResetGrants([])` → `null`; con todos `resetsLeft: 0` → `null`.
- suma `resetsLeft` (1 + 2 → 3) y elige el `endsAt` más temprano ignorando `null` e inválidos.
- `formatResetGrantsLabel({count:1, nearestEndsAt:null})` → `"1 reinicio disponible"`.
- `formatResetGrantsLabel({count:2, nearestEndsAt:"2026-11-30T12:00:00Z"})` → empieza con `"2 reinicios disponibles · vence el "` y contiene `"30"`.

### TS — `src/features/limits/LimitsSection.test.tsx`
- Actualizar `makeSnapshot` con `resetGrants: []`.
- `"shows the reset-grants row when grants are available"` — snapshot ok con semana + 1
  grant → `getByRole("status", { name: "Reinicios de límites" })` con texto `1 reinicio disponible`.
- `"hides the reset-grants row when there are no grants"` → `queryByRole("status", { name: "Reinicios de límites" })` es null.
- `"does not show the row when limits are unavailable"` — status `unavailable` + grants → no aparece.

Nunca contra `~/.claude`, `~/.local/share/claude`, la DB real ni el Keychain.

## 5. Criterios de aceptación

- [auto] La petición usa `?cedar_ember=1&skip_spend=1` y el UA `claude-cli/<ver> (external, cli)`:
  `cargo test user_agent_format detect_` + revisión del diff de `fetch_usage` (una sola `.get`, sin peticiones nuevas, `LIMITS_POLL_SECS` sigue en 300).
- [auto] Detección de versión con fallback: `cargo test cli_version` en `packages/app/src-tauri`.
- [auto] `cedar_ember` ausente/null/no elegible/malformado no rompe ni muestra grants:
  `cargo test cedar_ parse_with_malformed_cedar_ember_keeps_limits parse_without_cedar_ember_has_no_grants`.
- [auto] Selección de disponibles y orden: `cargo test eligible_mixed_grants_selects_available available_sorted_by_ends_at_none_last`.
- [auto] Notificaciones (primera vez, no repetir, por vencer una vez, mute, cap):
  `cargo test reset_grants`.
- [auto] Contrato IPC camelCase sin token: `cargo test snapshot_serializes_reset_grants_camel_case`.
- [auto] Fila del popover: `pnpm --filter tokenwatch exec vitest run src/features/limits`.
- [auto] `pnpm check` en verde desde la raíz.
- [navegador] `pnpm dev` → `http://localhost:1420`: el dashboard carga sin errores de
  consola; la sección de límites muestra el estado vacío habitual (sin fila de reinicios).
- [escritorio] Con `pnpm tauri:dev`, sesión de Claude Code iniciada y una cuenta con un
  reinicio sin usar:
  1. Al arrancar, en ≤ unos segundos llega una notificación "TokenWatch — Reinicio de
     límites disponible: <label> · vence el <fecha>. Reclámalo en claude.ai → Configuración → Uso".
  2. Abrir el popover: en la sección Límites, bajo los medidores, aparece
     "N reinicios disponibles · vence el <fecha>" con un punto azul; los medidores de
     sesión/semana siguen mostrando los mismos valores que antes del cambio.
  3. Salir y volver a abrir la app: **no** se repite la notificación.
  4. Activar "Silenciar alertas" en el popover y esperar al siguiente ciclo (≤ 5 min):
     no llega ninguna notificación de TokenWatch y la fila sigue visible.
  5. En una cuenta sin reinicios (o no elegible), la fila no aparece y no hay errores en la consola de `tauri dev`.
  Lógica cubierta por los criterios [auto] de notificaciones y fila.

## 6. Privacidad y secretos

- **Lee**: el token OAuth del Keychain (sin cambios, solo en `limits`); los **nombres** de
  entradas de `~/.local/share/claude/versions/` (no se abre ningún archivo ni se lee
  contenido; solo se usa el mayor `X.Y.Z`).
- **Red**: mismo host y path (`https://api.anthropic.com/api/oauth/usage`), misma
  frecuencia, una sola petición; cambian solo dos query params y el `User-Agent`. El
  token no cambia de destino. **Justificación del UA**: el endpoint solo devuelve el
  bloque `cedar_ember` a la superficie de Claude Code CLI (`ineligible_reason:"surface"`
  en otro caso); el UA solo declara el cliente/versión ya instalada en la máquina del
  usuario, no transporta datos del usuario. No hay endpoints nuevos ni cambios de CSP ni
  de capabilities.
- **SQLite**: nueva clave `meta.reset_grants_notified` con ids de grants (strings opacos
  del servidor, sin datos personales). No se persiste el token ni el UA.
- **IPC**: `LimitsSnapshot.resetGrants` con `id, label, resetsLeft, endsAt, type,
  usableNow`. El token nunca cruza (test `snapshot_serializes_reset_grants_camel_case`).
- **Logs**: solo `debug!` con la versión de CLI detectada y `debug!` genérico si
  `cedar_ember` no parsea (sin volcar el JSON). Nunca el token ni headers.
- **Riesgo**: Anthropic puede dejar de aceptar el UA o cambiar el bloque → degrada en
  silencio a "sin grants"; los límites siguen funcionando porque `cedar_ember` se parsea
  aparte.

## 7. Rendimiento y batería

- Polling: sin cambios (`LIMITS_POLL_SECS` 300 s, una petición; el refresco al abrir el
  popover tampoco cambia). La respuesta crece unos cientos de bytes; `skip_spend=1` la
  reduce.
- Detección de versión: un `read_dir` una vez por proceso (`OnceLock`).
- DB: un `meta_get` por ciclo y un `meta_set` solo cuando cambia el estado.
- Frontend: sin `setInterval`; la fila se actualiza con `limits-updated` vía `useLimits`.
  Cálculo O(n) sobre una lista de pocos grants.
- Ingesta de JSONL: no afectada.

## 8. Correctitud de datos

- Tokens/costo/dedup: no aplica.
- Tiempo: `ends_at` se compara en UTC contra `Utc::now()` (disponible si `ends_at > now`;
  por vencer si `0 < ends_at − now ≤ 3 días`, frontera inclusiva). La fecha mostrada es
  la del **día local** (`chrono::Local` en Rust, `toLocaleDateString("es")` en TS).
  `ends_at` no parseable → se trata como sin vencimiento (se muestra, sin fecha).
- Snapshot servido desde caché tras error transitorio: se re-evalúa la máquina de
  notificaciones con los mismos ids → idempotente, no duplica avisos. Entre el fetch y el
  render pueden pasar ≤ 5 min, por lo que un grant que vence en ese intervalo puede seguir
  visible hasta el próximo ciclo (aceptable).
- Migración: ninguna (se usa `meta`). Meta corrupta → estado vacío (re-notifica una vez).

## 9. Spec de openspec

Sin carpeta en `openspec/changes/` (regla del pipeline); se editan las specs vivas.

**`openspec/specs/usage-limits/spec.md`**

a) En "Requirement: Consulta de utilización de límites de Claude", reemplazar la primera
oración por:

> El sistema SHALL consultar `GET https://api.anthropic.com/api/oauth/usage?cedar_ember=1&skip_spend=1` con los headers `Authorization: Bearer <token>`, `anthropic-beta: oauth-2025-04-20` y `User-Agent: claude-cli/<versión> (external, cli)`, donde `<versión>` es la mayor versión `X.Y.Z` entre los nombres de entrada de `~/.local/share/claude/versions/` (o `2.1.328` si no hay ninguna), calculada una vez por ejecución de la app, y exponer un comando `query_limits` que devuelva la utilización de la ventana de **sesión de 5h**, de la **semana** (total) y de los **límites semanales por modelo**.

b) Agregar al final:

```markdown
### Requirement: Reinicios gratuitos de límites disponibles
El sistema SHALL leer el bloque `cedar_ember` de la misma respuesta de uso (sin peticiones adicionales) y exponer en el snapshot de límites (`resetGrants`) la lista de reinicios disponibles, cada uno con `id`, `label`, `resetsLeft`, `endsAt`, `type` y `usableNow`, ordenada por `endsAt` ascendente (sin vencimiento al final). Un reinicio MUST considerarse disponible solo si el bloque es elegible (`eligible: true`), el reinicio tiene `id`, no está pausado, `resets_left > 0` y su `ends_at` no ha pasado. Si el bloque falta, es `null`, tiene un tipo inesperado o no es elegible, la lista MUST quedar vacía sin error y sin afectar a las demás ventanas. El sistema MUST NOT reclamar reinicios.

#### Scenario: Bloque ausente o no elegible
- **WHEN** la respuesta no trae `cedar_ember`, lo trae en `null` o con `eligible: false`
- **THEN** `resetGrants` está vacío y sesión y semana se devuelven normalmente

#### Scenario: Reinicios mezclados
- **WHEN** `cedar_ember` es elegible y trae un reinicio disponible, uno pausado, uno con `resets_left: 0` y uno vencido
- **THEN** `resetGrants` contiene solo el disponible

#### Scenario: Bloque mal formado
- **WHEN** `cedar_ember` tiene un tipo inesperado o un reinicio mal formado
- **THEN** se ignora lo mal formado y el resto de la respuesta se parsea sin error

### Requirement: Notificaciones de reinicios gratuitos
El sistema SHALL emitir una notificación de macOS (título "TokenWatch") por cada reinicio disponible que no se haya notificado antes, incluidos los existentes la primera vez que se ejecuta, con el texto "Reinicio de límites disponible: <label> · vence el <fecha>. Reclámalo en claude.ai → Configuración → Uso". El sistema SHALL emitir además, una sola vez por reinicio, la notificación "Reinicio de límites por vencer: <label> · vence el <fecha>. Reclámalo en claude.ai → Configuración → Uso" cuando falten 3 días o menos para su vencimiento; si un reinicio aparece por primera vez ya dentro de ese plazo, MUST enviarse una sola notificación. Los reinicios notificados MUST persistir entre reinicios de la app. Con "silenciar alertas" activo no se emite ninguna de estas notificaciones y los reinicios se dan por notificados.

#### Scenario: Primer arranque con reinicios sin usar
- **WHEN** la app consulta por primera vez y hay un reinicio disponible
- **THEN** emite la notificación de reinicio disponible

#### Scenario: Sin repetición
- **WHEN** un reinicio ya notificado sigue disponible en consultas posteriores o tras reiniciar la app
- **THEN** no se emite una nueva notificación de disponible

#### Scenario: Por vencer
- **WHEN** un reinicio ya notificado pasa a vencer en 3 días o menos
- **THEN** se emite una sola notificación de reinicio por vencer

#### Scenario: Alertas silenciadas
- **WHEN** las alertas están silenciadas y aparece un reinicio disponible
- **THEN** no se emite notificación
```

**`openspec/specs/menubar-popover/spec.md`** — agregar al final:

```markdown
### Requirement: Fila de reinicios gratuitos disponibles
Cuando el snapshot de límites esté disponible e incluya reinicios disponibles, la sección de límites SHALL mostrar una fila compacta "N reinicios disponibles · vence el <fecha>" (singular "1 reinicio disponible"), donde N es la suma de los reinicios restantes y la fecha (día y mes abreviado, hora local) corresponde al reinicio que vence antes. Si ese reinicio no tiene vencimiento, MUST omitirse "· vence el <fecha>". Si no hay reinicios disponibles o los límites no están disponibles, la fila MUST NOT mostrarse.

#### Scenario: Reinicios disponibles
- **WHEN** el snapshot incluye dos reinicios disponibles que vencen el 30 nov y el 15 dic
- **THEN** la sección de límites muestra "2 reinicios disponibles · vence el 30 nov"

#### Scenario: Sin reinicios
- **WHEN** el snapshot no incluye reinicios disponibles
- **THEN** la fila no se muestra
```

## 10. Título de PR propuesto

`feat(limits): notify and show available free limit resets`

## Fuera de alcance / seguimiento

- Reclamar reinicios (`POST …/reset_rate_limits`): sigue siendo manual en claude.ai.
- Abrir `https://claude.ai/settings/usage` al hacer click en la fila: requiere agregar
  `tauri-plugin-opener` + capability `opener:allow-open-url` acotada a esa URL; issue aparte.
- Detección de caídas de uso (descartada).
- Usar `next_grant_id`, `cooldown_until`, `clears`, `gives_back_weekly`, `claimable` en
  la UI (se modelan en el parseo pero no se muestran).
- Re-detectar la versión de Claude Code sin reiniciar la app.
