## Context

TokenWatch ingiere directamente `~/.claude/projects/**/*.jsonl` (no usa `ccusage`). Por cada línea `assistant` con `message.usage`, `ingest_file` (`packages/app/src-tauri/src/ingest/mod.rs`, ~línea 311) llama a `derive_project_name(record.cwd.as_deref())` y persiste `project_name` junto al `cwd` crudo en `project_path`.

Estado actual de `derive_project_name` (`ingest/mod.rs:82-128`), reglas en orden:

| Línea | Regla | Resultado |
|---|---|---|
| 83-86 | `cwd` `None` o vacío | `unknown` |
| 90-104 | sin componentes (tras filtrar `RootDir`/`Prefix`) | `unknown` |
| 106-109 | `components.contains(&"worktrees")` | `unknown` ← **causa raíz** |
| 111-119 | par adyacente `conductor`,`workspaces` → componente siguiente; si no hay siguiente (ruta termina en `conductor/workspaces`) | repo / `unknown` |
| 121-125 | fallback | últimos 1–2 segmentos |

Paseo ubica cada checkout en `~/.paseo/worktrees/<id>/<slug>` (y las sesiones a veces corren en subdirectorios, p. ej. `.../upbeat-bear/dev-wiki`). Todos contienen `worktrees` → `unknown`.

Migraciones (`src/db/mod.rs`, `fn migrate`): v1 DDL; v2, v3, v4 re-ejecutan `backfill_project_names`; v5 DDL de grupos; v6 y v7 `recompute_costs`. **Versión vigente: 7** (confirmado en código y en la DB local). La siguiente es **v8**. `backfill_project_names` (`db/mod.rs:209`) lee `(dedup_key, project_path)` de todas las filas y hace un `UPDATE` por fila dentro de una transacción; llama a `crate::ingest::derive_project_name(Some(&project_path))`. Su doc-comment define el **contrato D19**: cualquier re-derivación que cambie nombres debe remapear `project_group_members.project_name`.

Llamadores de las funciones que cambian de firma:
- `derive_project_name`: `ingest_file` (`ingest/mod.rs:311`), `backfill_project_names` (`db/mod.rs:221`), tests en `ingest/mod.rs` (`test_derive_*`).
- `ingest_from`: `ingest()` (`ingest/mod.rs:163`) y 7 tests en `ingest/mod.rs` (líneas ~523, 545, 559, 584, 597, 615, 620, 633).
- `ingest::ingest(&conn)`: `src/usage/mod.rs:150` (`refresh_usage`) y `:536` (`run_refresh_and_emit`, polling cada `POLL_INTERVAL_SECS`). Su firma **no** cambia.

### Formato observado de `~/.paseo/projects/workspaces.json` (inspeccionado read-only)

Archivo interno de Paseo, no documentado. Es un **array JSON** de objetos (373 en esta máquina: 367 `kind: "worktree"`, 6 `kind: "local_checkout"`). Campos relevantes:

```json
{
  "workspaceId": "wks_…",
  "projectId": "prj_…",
  "cwd": "/Users/x/.paseo/worktrees/0ttsehts/white-quail",
  "kind": "worktree",
  "worktreeRoot": "/Users/x/.paseo/worktrees/0ttsehts/white-quail",
  "mainRepoRoot": "/Users/x/dev/meki/console",
  "isPaseoOwnedWorktree": true,
  "archivedAt": "2026-09-01T20:48:05.139Z" | null,
  "displayName": "white-quail", "branch": "…", "baseBranch": "…", "title": "…" | null, …
}
```

- Los worktrees archivados siguen presentes (`archivedAt` no nulo) → cubren el histórico.
- En `local_checkout`, `mainRepoRoot` es `null` y `cwd == worktreeRoot` apunta al repo real (no está bajo `.paseo/worktrees`, por lo que no aplica).
- En esta máquina `cwd == worktreeRoot` en todas las entradas, pero se indexan ambos por robustez.
- El `.git` de un worktree vivo contiene `gitdir: /Users/x/dev/freelance/tokenwatch/.git/worktrees/resolute-seal`.

## Goals / Non-Goals

**Goals:**
- Atribuir las sesiones de Paseo al repositorio principal (`mainRepoRoot`), con el mismo criterio de producto que Conductor (agrupar por repo real).
- Mantener `unknown` sólo para worktrees efímeros de agente (`.claude/worktrees/agent-*`, incluidos los que viven dentro de workspaces de Conductor).
- Mantener `derive_project_name` pura: toda la E/S de archivos vive en un resolver cargado una vez y pasado como parámetro.
- Re-atribuir las filas históricas mediante la migración v8.
- Degradación elegante: nunca panic, nunca abortar la ingesta ni la migración por culpa de Paseo.

**Non-Goals:**
- No se cambia el esquema de la DB (sin columnas nuevas).
- No se soportan otras herramientas de worktrees (p. ej. `git worktree` manual fuera de Paseo) más allá de que dejan de caer en `unknown` por la regla amplia.
- No se remapea `project_group_members` (ver Decisión 7).
- No se agrega UI ni configuración.
- No se asume que el segmento `<id>` de Paseo identifique un repo.

## Decisions

### D1. Nuevo tipo `PaseoIndex` en `ingest/mod.rs`

```rust
pub struct PaseoIndex {
    /// worktreeRoot / cwd normalizado (sin "/" final) → mainRepoRoot.
    roots: std::collections::HashMap<String, String>,
    /// Cache del fallback `.git` por worktree root (evita leer disco por línea).
    git_cache: std::cell::RefCell<std::collections::HashMap<String, Option<String>>>,
}
```

API:
- `PaseoIndex::empty() -> Self`
- `PaseoIndex::from_json_str(json: &str) -> Self` — parsea con `serde_json` a `Vec<RawPaseoWorkspace>` (struct lax, todos los campos `Option<String>`, `#[serde(default)]`, ignora desconocidos). Si el JSON no parsea o no es un array → `tracing::warn!` + índice vacío. Para cada entrada con `mainRepoRoot` no vacío inserta `worktreeRoot` y `cwd` (si no vacíos), normalizados quitando `/` final.
- `PaseoIndex::load_from_home(home: &Path) -> Self` — lee `home/.paseo/projects/workspaces.json`; si no existe → `debug!` + vacío; si falla la lectura → `warn!` + vacío.
- `PaseoIndex::load_default() -> Self` — resuelve home con la misma lógica que `claude_projects_dir` (extraer helper privado `home_dir() -> Option<PathBuf>` y reusarlo en ambos) y llama a `load_from_home`; sin home → vacío.
- `fn resolve_main_repo(&self, worktree_root: &str, cwd: &str) -> Option<String>` (privada al módulo): 
  1. Lookup por prefijo más largo: probar `cwd`, luego quitar el último componente repetidamente mientras la ruta siga siendo ≥ `worktree_root`; el primer hit en `roots` gana. (HashMap + recorte por componentes = O(profundidad), evita escaneo lineal por línea.)
  2. Si no hay hit: `git_cache` por `worktree_root`; si no está cacheado, `read_gitdir_main_repo(Path::new(worktree_root))` y cachear el resultado (incluido `None`).

**Por qué `RefCell` y no `&mut`**: permite pasar `&PaseoIndex` por `ingest_from`/`ingest_file`/`backfill_project_names` sin propagar `&mut`. Todo el uso es secuencial en un hilo (la conexión está detrás de un `Mutex`); `PaseoIndex` no necesita ser `Sync`.

**Alternativa descartada**: leer `workspaces.json` dentro de `derive_project_name` → impuro, lee ~300 KB por línea, no testeable.

### D2. Fallback por archivo `.git`

`fn read_gitdir_main_repo(worktree_root: &Path) -> Option<String>`: lee `worktree_root/.git`; si no es archivo regular o no se puede leer → `None`. Contenido `trim()`; debe empezar con `gitdir:`; tomar el resto `trim()`; buscar `"/.git/"`; devolver lo anterior. Si no contiene `/.git/` → `None`. Sólo funciona si el worktree sigue en disco (los archivados suelen borrarse), por eso es secundario.

### D3. Detección de Paseo por componentes, no por home

En `derive_project_name`, tras construir `components` y **antes** de la regla de worktrees efímeros: buscar `i` tal que `components[i] == ".paseo" && components[i+1] == "worktrees"`. Entonces `id = components.get(i+2)`, `slug = components.get(i+3)`.
- Si falta `slug` (ruta `.paseo/worktrees` o `.paseo/worktrees/<id>`) → `unknown`.
- `worktree_root = "/" + components[..=i+3].join("/")`; `cwd_norm = "/" + components.join("/")`.
- `paseo.resolve_main_repo(&worktree_root, &cwd_norm)` → `Some(main)` → `derive_non_paseo_name(&main)`; `None` → `slug.to_string()`.

Detectar por el par de componentes (y no comparando con `$HOME`) mantiene la función pura y permite tests con `tempfile::TempDir` (cuyo path contiene `.paseo/worktrees/...` si lo creamos así). El JSON real usa rutas absolutas idénticas al `cwd` de Claude, por lo que la reconstrucción con `/` + componentes coincide.

### D4. Nombre a partir de `mainRepoRoot`: reusar las reglas no-Paseo

Refactor: extraer el cuerpo actual (desde "sin componentes" hasta el fallback) a `fn derive_from_components(components: &[&str]) -> String` y crear `fn derive_non_paseo_name(path: &str) -> String` que construye componentes y llama a `derive_from_components`. Así `mainRepoRoot = /Users/x/dev/freelance/tokenwatch` → `freelance/tokenwatch` (regla de últimos dos), y si algún día `mainRepoRoot` fuese un workspace de Conductor también se resuelve bien. Sin recursión hacia Paseo (evita ciclos).

### D5. Regla de worktrees efímeros acotada

Reemplazar `components.contains(&"worktrees")` por:
- par adyacente `components[i] == ".claude" && components[i+1] == "worktrees"`, **o**
- `components[i] == "worktrees"` y `components[i+1].starts_with("agent-")`.

Se mantiene **antes** de la regla de Conductor, así `/Users/x/conductor/workspaces/tub2/kelowna-v1/FRONT/.claude/worktrees/agent-…` sigue en `unknown` (comportamiento actual; único caso Conductor en la DB). La segunda condición (más allá del brief, que pedía sólo `.claude`,`worktrees`) preserva los tests existentes `test_derive_worktrees_component` (`/Users/x/worktrees/agent-1234/packages/app`) y `test_v3_backfill_corrects_worktree_names` (`/Users/x/dev/tub2/worktrees/agent-1234`), y conserva la intención original ("worktrees de agente") sin atrapar Paseo (que además se evalúa antes). Otras rutas con `worktrees` no-agente pasan a la derivación por segmentos.

### D6. Firmas nuevas

- `pub fn derive_project_name(cwd: Option<&str>, paseo: &PaseoIndex) -> String`
- `pub fn ingest_from(conn: &Connection, base_dir: &Path, paseo: &PaseoIndex) -> anyhow::Result<IngestSummary>`
- `fn ingest_file(conn: &Connection, path: &Path, paseo: &PaseoIndex) -> anyhow::Result<(bool, u64)>`
- `pub fn ingest(conn)` sin cambio de firma: crea `let paseo = PaseoIndex::load_default();` **una vez** por corrida y lo pasa a `ingest_from`. Recargar en cada polling es deliberado: Paseo crea worktrees nuevos constantemente; el costo (~300 KB JSON) es despreciable frente al intervalo de polling.
- `fn backfill_project_names(conn: &Connection, paseo: &crate::ingest::PaseoIndex) -> SqlResult<()>` en `db/mod.rs`.

**Alternativa descartada**: mantener `derive_project_name(cwd)` como wrapper con índice vacío → riesgo de usarlo por error en producción y obtener slugs en vez de repos.

### D7. Migración v8 y backfill optimizado

- `apply_v2`, `apply_v3`, `apply_v4` pasan `&PaseoIndex::empty()` (preservan su semántica histórica; v8 corre después y corrige con el índice real).
- `fn apply_v8(conn) -> SqlResult<()>`: `let paseo = crate::ingest::PaseoIndex::load_default(); backfill_project_names(conn, &paseo)`. En `migrate`, bloque `if version < 8 { apply_v8(conn)?; set_meta(conn, "schema_version", "8")?; }` **sin** transacción externa (backfill abre la suya, igual que v4/v6/v7), escribiendo el literal `"8"` (convención D13).
- `backfill_project_names` pasa a: `SELECT dedup_key, project_path, project_name`; cache `HashMap<String, String>` de nombre derivado por `project_path` (en la DB hay pocos cientos/miles de paths distintos frente a ~120k filas); `UPDATE ... WHERE dedup_key = ?` **sólo si** el nombre nuevo difiere del actual. Mismo resultado e idempotencia, muchas menos escrituras. Usar `tx.prepare_cached` para el `UPDATE`.
- **D19 / `project_group_members`**: no se remapea. El nombre de origen que cambia es esencialmente `unknown` (→ N repos distintos, mapeo 1:N no representable) y `unknown` sigue existiendo para los worktrees `.claude`. Los nombres destino (`freelance/tokenwatch`, etc.) ya existen como proyectos de checkouts locales, así que sus membresías existentes empiezan a recibir el uso de Paseo, que es lo deseado. Se documenta como excepción explícita al contrato D19 en el doc-comment de `apply_v8`. (En esta máquina hay 0 membresías de `unknown`.)
- Si `workspaces.json` no existe en el momento de migrar, v8 igual se marca aplicada: las filas Paseo quedan con repo por `.git` o con slug. Aceptado (no se reintenta; ver Open Questions).

### D8. Tests sin tocar el home real

Todos los tests usan `PaseoIndex::empty()`, `PaseoIndex::from_json_str(...)` o `PaseoIndex::load_from_home(tmp.path())`. Los tests que pasan por `open_at(":memory:")` ejecutan `apply_v8` → `load_default()` sobre una tabla vacía: inocuo (sólo lectura; sin filas no hay escrituras).

## Risks / Trade-offs

- [Formato de `workspaces.json` cambia o se mueve] → struct lax con campos opcionales; JSON inválido → índice vacío + `warn!`; cadena de fallback `.git` → slug. Nunca `unknown` para Paseo con slug presente, nunca panic.
- [Worktree archivado y borrado, ausente del índice] → cae a slug: el uso queda separado por slug en lugar de agrupado, pero visible y no mezclado en `unknown`.
- [Se asume `<id>` no es por repo] → nunca se usa `<id>` para derivar el nombre.
- [Tiempo de migración v8 sobre ~120k filas] → derivación cacheada por `project_path` y `UPDATE` sólo si cambia (~113k updates por PK dentro de una sola transacción, del orden de segundos); corre una sola vez al abrir la DB.
- [Colisión de nombres]: un repo `dev/meki/backend` y otro `work/meki/backend` comparten `meki/backend` → limitación preexistente de la regla de últimos dos segmentos; no se aborda.
- [`RefCell` hace `PaseoIndex: !Sync`] → sólo se usa dentro de una llamada síncrona; si en el futuro se comparte entre hilos habría que pasar a `Mutex`.
- [Rutas no-agente con `worktrees` cambian de `unknown` a segmentos] → cambio intencional; la DB local no tiene casos así.

## Migration Plan

1. Al abrir la DB (`open_at`), `migrate` aplica v8 si `schema_version < 8`: carga `PaseoIndex` del home y re-ejecuta el backfill.
2. Las ingestas siguientes usan el índice recargado en cada corrida.
3. Rollback: revertir el binario deja `schema_version = 8` con nombres ya corregidos; la versión anterior ignora versiones mayores (sólo compara `<`), por lo que sigue funcionando, aunque las filas Paseo nuevas volverían a `unknown`.

## Open Questions

- ¿Conviene una migración/recálculo futuro si aparecen filas Paseo con slug porque `workspaces.json` no estaba disponible al migrar? Por ahora no; la ingesta incremental ya resuelve correctamente las filas nuevas.
- ¿Se quiere extender el mismo enfoque (leer `.git`) a worktrees genéricos fuera de Paseo? Fuera de alcance.
