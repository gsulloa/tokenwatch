> Todas las rutas son relativas a `packages/app/src-tauri/`. Los comandos `cargo` se corren desde `packages/app/src-tauri`; los `pnpm` desde la raíz del repo. Ver `design.md` (D1–D8) para el porqué de cada decisión. No agregar dependencias nuevas (`serde`, `serde_json`, `tracing`, `tempfile` ya están).

## 1. Refactor puro de `derive_project_name` (sin cambio de comportamiento todavía)

- [x] 1.1 En `src/ingest/mod.rs`, extraer un helper privado `fn path_components(path: &str) -> Vec<&str>` con la lógica actual de líneas ~89-100 (filtra `Component::RootDir` y `Component::Prefix`, `filter_map(to_str)`).
- [x] 1.2 Extraer `fn derive_from_components(components: &[&str]) -> String` con el cuerpo actual desde "si `components.is_empty()` → `unknown`" hasta el fallback de últimos 1–2 segmentos (incluye la regla de worktrees y la de `conductor`/`workspaces`, sin modificarlas aún).
- [x] 1.3 Agregar `fn derive_non_paseo_name(path: &str) -> String` que hace `derive_from_components(&path_components(path))`.
- [x] 1.4 Correr `cargo test ingest` y confirmar que todos los tests existentes siguen verdes antes de seguir.

## 2. `PaseoIndex` (carga de `workspaces.json` + fallback `.git`)

- [x] 2.1 En `src/ingest/mod.rs`, agregar `use std::{cell::RefCell, collections::HashMap};` y una sección `// Paseo worktree resolution`.
- [x] 2.2 Definir `#[derive(Debug, Deserialize, Default)] struct RawPaseoWorkspace { #[serde(default, rename = "worktreeRoot")] worktree_root: Option<String>, #[serde(default)] cwd: Option<String>, #[serde(default, rename = "mainRepoRoot")] main_repo_root: Option<String> }` (campos desconocidos ignorados; no usar `deny_unknown_fields`).
- [x] 2.3 Definir `#[derive(Debug, Default)] pub struct PaseoIndex { roots: HashMap<String, String>, git_cache: RefCell<HashMap<String, Option<String>>> }` con doc-comment que explique que es interno de Paseo y no `Sync`.
- [x] 2.4 Agregar `fn normalize(p: &str) -> String` que hace `trim()` y quita `/` finales (pero deja `"/"` si es la raíz).
- [x] 2.5 Implementar `pub fn empty() -> Self` (= `Default::default()`).
- [x] 2.6 Implementar `pub fn from_json_str(json: &str) -> Self`: `serde_json::from_str::<Vec<RawPaseoWorkspace>>(json)`; en `Err(e)` → `tracing::warn!(error = %e, "invalid Paseo workspaces.json; ignoring")` y devolver `empty()`. Por cada entrada con `main_repo_root` `Some` y no vacío (tras `trim`): insertar `normalize(worktree_root)` y `normalize(cwd)` (los que sean `Some` y no vacíos) → `normalize(main_repo_root)` en `roots`.
- [x] 2.7 Extraer en `src/ingest/mod.rs` un helper privado `fn home_dir() -> Option<std::path::PathBuf>` con la lógica actual de `claude_projects_dir` (`HOME` env var o `directories::UserDirs`), y reescribir `claude_projects_dir` para usarlo (sin cambiar su comportamiento).
- [x] 2.8 Implementar `pub fn load_from_home(home: &Path) -> Self`: ruta `home.join(".paseo").join("projects").join("workspaces.json")`; si no existe → `tracing::debug!` y `empty()`; si `std::fs::read_to_string` falla → `tracing::warn!` y `empty()`; si no → `from_json_str(&contents)`. Nunca `unwrap`/`expect`.
- [x] 2.9 Implementar `pub fn load_default() -> Self`: `match home_dir() { Some(h) => Self::load_from_home(&h), None => Self::empty() }`.
- [x] 2.10 Implementar `fn read_gitdir_main_repo(worktree_root: &Path) -> Option<String>`: `let git = worktree_root.join(".git"); if !git.is_file() { return None; }`; `read_to_string(...).ok()?`; `trim()`; `strip_prefix("gitdir:")?`; `trim()`; `find("/.git/")` → devolver `Some(s[..idx].to_owned())` si `idx > 0`, si no `None`.
- [x] 2.11 Implementar `fn resolve_main_repo(&self, worktree_root: &str, cwd: &str) -> Option<String>`:
  1. `let mut probe = cwd.to_owned();` en loop: si `self.roots.get(&probe)` → `return Some(v.clone())`; si `probe.len() <= worktree_root.len()` → `break`; si no, recortar el último componente (`probe.rfind('/')` → `probe.truncate(idx)`; si no hay `/`, `break`).
  2. Fallback `.git`: si `self.git_cache.borrow().get(worktree_root)` existe → devolver su clon; si no, `let r = read_gitdir_main_repo(Path::new(worktree_root));` insertar en `git_cache` (`borrow_mut`) y devolver `r`.

## 3. Nueva lógica de derivación

- [x] 3.1 Cambiar la firma a `pub fn derive_project_name(cwd: Option<&str>, paseo: &PaseoIndex) -> String`. Cuerpo: `cwd` `None`/vacío → `unknown`; `let components = path_components(cwd);` si vacío → `unknown`; luego la regla Paseo (3.2); luego `derive_from_components(&components)`.
- [x] 3.2 Regla Paseo (antes de cualquier otra regla de componentes): buscar el primer `i` con `components[i] == ".paseo" && components.get(i + 1) == Some(&"worktrees")`. Si existe: si `components.get(i + 3)` es `None` → `return "unknown"`; `let slug = components[i + 3];` `let worktree_root = format!("/{}", components[..=i + 3].join("/"));` `let cwd_norm = format!("/{}", components.join("/"));` `return match paseo.resolve_main_repo(&worktree_root, &cwd_norm) { Some(main) => derive_non_paseo_name(&main), None => slug.to_owned() };`.
- [x] 3.3 En `derive_from_components`, reemplazar `if components.contains(&"worktrees")` por: existe `i` tal que (`components[i] == ".claude" && components.get(i + 1) == Some(&"worktrees")`) **o** (`components[i] == "worktrees" && components.get(i + 1).is_some_and(|c| c.starts_with("agent-"))`) → `"unknown"`. Mantener esta regla **antes** de la regla `conductor`/`workspaces`.
- [x] 3.4 Actualizar el doc-comment de `derive_project_name` con las reglas nuevas (orden: vacío → Paseo → worktree de agente → Conductor → fallback) y ejemplos: `/Users/x/.paseo/worktrees/3qkyl933/resolute-seal` → `freelance/tokenwatch` (con índice), `.../.claude/worktrees/agent-abcd` → `unknown`.

## 4. Propagar el índice por la ingesta

- [x] 4.1 Cambiar `fn ingest_file(conn: &Connection, path: &Path)` a `fn ingest_file(conn: &Connection, path: &Path, paseo: &PaseoIndex)` y en la línea ~311 usar `derive_project_name(record.cwd.as_deref(), paseo)`.
- [x] 4.2 Cambiar `pub fn ingest_from(conn: &Connection, base_dir: &Path)` a `pub fn ingest_from(conn: &Connection, base_dir: &Path, paseo: &PaseoIndex)` y pasar `paseo` a `ingest_file`. Actualizar su doc-comment.
- [x] 4.3 En `pub fn ingest(conn)` (firma sin cambios): después de resolver `base`, crear `let paseo = PaseoIndex::load_default();` una sola vez y llamar `ingest_from(conn, &base, &paseo)`. Confirmar que `src/usage/mod.rs` (líneas ~150 y ~536) compila sin cambios.
- [x] 4.4 Actualizar todas las llamadas a `ingest_from(&conn, ...)` en los tests de `src/ingest/mod.rs` (≈7 llamadas: `test_ingest_skips_synthetic_model`, `test_ingest_dedup_same_id_twice`, `test_ingest_project_name_unknown_when_no_cwd`, `test_incremental_ingest_grown_file_no_duplicates`, `test_incremental_unchanged_file_skipped`, `test_real_fixture_file`, y la de `test_ingest_fixture_produces_expected_rows` si la hay) agregando `&PaseoIndex::empty()` como tercer argumento.
- [x] 4.5 Actualizar todas las llamadas `derive_project_name(X)` en los tests existentes a `derive_project_name(X, &PaseoIndex::empty())`, sin cambiar sus valores esperados.

## 5. Backfill y migración v8 (`src/db/mod.rs`)

- [x] 5.1 Cambiar `fn backfill_project_names(conn: &Connection)` a `fn backfill_project_names(conn: &Connection, paseo: &crate::ingest::PaseoIndex) -> SqlResult<()>`:
  - `SELECT dedup_key, project_path, project_name FROM usage_events` → `Vec<(String, String, String)>`.
  - `let mut cache: HashMap<String, String> = HashMap::new();` (`use std::collections::HashMap;`).
  - Abrir `let tx = conn.unchecked_transaction()?;` y `let mut upd = tx.prepare_cached("UPDATE usage_events SET project_name = ?1 WHERE dedup_key = ?2")?;`.
  - Por fila: `let new_name = cache.entry(project_path.clone()).or_insert_with(|| crate::ingest::derive_project_name(Some(&project_path), paseo)).clone();` y sólo si `new_name != current_name` → `upd.execute(params![new_name, dedup_key])?`.
  - `drop(upd); tx.commit()?;`.
  - Mantener el doc-comment D19 y agregar que sólo escribe filas cuyo nombre cambia.
- [x] 5.2 Actualizar `apply_v2`, `apply_v3`, `apply_v4` para llamar `backfill_project_names(conn, &crate::ingest::PaseoIndex::empty())` (comentario: "v8 re-runs with the real Paseo index").
- [x] 5.3 Agregar `fn apply_v8(conn: &Connection) -> SqlResult<()>` con doc-comment: re-atribuye filas de worktrees de Paseo (antes `unknown`) al repo principal usando `~/.paseo/projects/workspaces.json`, fallback `.git` y slug; **excepción explícita a D19**: no remapea `project_group_members` porque el nombre de origen es `unknown` (mapeo 1:N) y `unknown` sigue existiendo para `.claude/worktrees`. Cuerpo: `let paseo = crate::ingest::PaseoIndex::load_default(); backfill_project_names(conn, &paseo)`.
- [x] 5.4 En `migrate`, después del bloque `version < 7`, agregar `if version < 8 { // backfill opens its own transaction, so do NOT wrap here.  apply_v8(conn)?; set_meta(conn, "schema_version", "8")?; }` (literal `"8"`).
- [x] 5.5 Actualizar los asserts de versión `"7"` → `"8"` en `src/db/mod.rs` (`test_schema_creation`, ~línea 444) y en `src/budgets/mod.rs` (`test_v5_tables_exist_after_migration`, ~línea 689). Verificar con `grep -rn '"7"' src` que no quedan asserts de versión desactualizados.

## 6. Tests nuevos en `src/ingest/mod.rs` (módulo `tests`)

- [x] 6.1 Agregar constante de test `PASEO_JSON` con un array de 3 entradas: (a) `worktreeRoot`=`cwd`=`/Users/x/.paseo/worktrees/3qkyl933/resolute-seal`, `mainRepoRoot`=`/Users/x/dev/freelance/tokenwatch`, `kind`="worktree", `archivedAt`=null; (b) `worktreeRoot`=`cwd`=`/Users/x/.paseo/worktrees/abcd1234/upbeat-bear`, `mainRepoRoot`=`/Users/x/dev/meki/backend`, `archivedAt`="2026-10-01T00:00:00Z" y un campo extra desconocido (`"foo": 1`); (c) `kind`="local_checkout", `cwd`=`/Users/x/dev/meki/console`, `mainRepoRoot`=null.
- [x] 6.2 `test_derive_paseo_worktree_root_uses_main_repo`: con `PaseoIndex::from_json_str(PASEO_JSON)`, `derive_project_name(Some("/Users/x/.paseo/worktrees/3qkyl933/resolute-seal"), &idx)` == `"freelance/tokenwatch"`.
- [x] 6.3 `test_derive_paseo_subdir_uses_main_repo`: `"/Users/x/.paseo/worktrees/abcd1234/upbeat-bear/dev-wiki"` → `"meki/backend"` (también cubre entrada archivada y campo desconocido); y `"/Users/x/.paseo/worktrees/abcd1234/upbeat-bear/a/b/c"` → `"meki/backend"`.
- [x] 6.4 `test_derive_paseo_trailing_slash_in_index`: índice con `worktreeRoot` terminado en `/` (`".../zz/slug/"`) resuelve el `cwd` `".../zz/slug"` a su repo (valida `normalize`).
- [x] 6.5 `test_derive_paseo_git_file_fallback`: `TempDir`; crear `<tmp>/.paseo/worktrees/id1/brave-otter/` y escribir el archivo `.git` con `"gitdir: /Users/x/dev/meki/backend/.git/worktrees/brave-otter\n"`; con `PaseoIndex::empty()`, `derive_project_name(Some(<tmp>/.paseo/worktrees/id1/brave-otter), ..)` == `"meki/backend"`; y el subdir `<...>/brave-otter/src` también == `"meki/backend"`.
- [x] 6.6 `test_derive_paseo_slug_last_resort`: con `PaseoIndex::empty()` y una ruta inexistente `"/nonexistent-tw-test/.paseo/worktrees/zz99/brave-otter"` → `"brave-otter"`; con `.../brave-otter/sub/dir` → `"brave-otter"`.
- [x] 6.7 `test_derive_paseo_incomplete_path_is_unknown`: `"/Users/x/.paseo/worktrees"` y `"/Users/x/.paseo/worktrees/zz99"` → `"unknown"`.
- [x] 6.8 `test_derive_paseo_git_file_without_marker_falls_back_to_slug`: `.git` con contenido `"gitdir: /weird/path"` (sin `/.git/`) → slug; `.git` como **directorio** → slug.
- [x] 6.9 `test_derive_claude_agent_worktrees_still_unknown`: con el índice de 6.1, `"/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd"` → `"unknown"`, `"/Users/x/conductor/workspaces/tub2/kelowna-v1/FRONT/.claude/worktrees/agent-ad31"` → `"unknown"`, `"/Users/x/worktrees/agent-1234/packages/app"` → `"unknown"`.
- [x] 6.10 `test_derive_generic_worktrees_component_uses_fallback`: `"/Users/x/dev/worktrees/feature-x"` → `"worktrees/feature-x"`.
- [x] 6.11 `test_paseo_index_malformed_json_is_empty`: `from_json_str("{not json")`, `from_json_str("{\"a\":1}")` (objeto, no array) y `from_json_str("[{\"mainRepoRoot\": 5}]")` (tipo inválido) no hacen panic, y luego `derive_project_name(Some("/nonexistent-tw-test/.paseo/worktrees/i/slug-a"), &idx)` == `"slug-a"`.
- [x] 6.12 `test_paseo_index_load_from_home`: `TempDir` como home; (a) sin archivo → índice vacío (la ruta Paseo devuelve slug); (b) escribir `PASEO_JSON` en `<tmp>/.paseo/projects/workspaces.json` → `load_from_home(tmp.path())` resuelve `resolute-seal` a `"freelance/tokenwatch"`; (c) archivo con basura → no panic, devuelve slug.
- [x] 6.13 `test_ingest_paseo_cwd_attributed_to_main_repo`: fixture JSONL (con `make_fixture_dir`) con una línea `assistant` válida cuyo `cwd` es `/Users/x/.paseo/worktrees/3qkyl933/resolute-seal`; `ingest_from(&conn, tmp.path(), &PaseoIndex::from_json_str(PASEO_JSON))`; asserts: 1 fila; `project_name == "freelance/tokenwatch"`; `project_path` == el cwd crudo.

## 7. Tests nuevos en `src/db/mod.rs` (módulo `tests`)

- [x] 7.1 `test_v8_backfill_reattributes_paseo_unknown_rows`: `test_conn()`; insertar con `insert_event` 4 filas: (a) Paseo en índice con `project_name = "unknown"`; (b) Paseo subdir en índice con `"unknown"`; (c) `.claude/worktrees/agent-x` con `"unknown"`; (d) Conductor `/Users/x/conductor/workspaces/tub2/chengdu-v4` con `"tub2"`. Llamar `backfill_project_names(&conn, &crate::ingest::PaseoIndex::from_json_str(<json con las rutas de a y b>))`. Asserts: (a) y (b) → repo esperado; (c) sigue `"unknown"`; (d) sigue `"tub2"`; `project_path` de todas sin cambios.
- [x] 7.2 `test_v8_backfill_idempotent`: tras 7.1, re-ejecutar el backfill con el mismo índice → mismos valores, sin error; verificar con `conn.changes()` o comparando un `SELECT` completo antes/después.
- [x] 7.3 `test_v8_backfill_does_not_touch_group_members`: insertar un grupo y una membresía `project_name = "unknown"` en `project_group_members`; correr el backfill; la membresía sigue intacta.
- [x] 7.4 Confirmar que `test_v2_backfill_*` y `test_v3_backfill_corrects_worktree_names` siguen pasando sin cambiar sus valores esperados (la ruta `/Users/x/dev/tub2/worktrees/agent-1234` sigue → `unknown` por la regla `worktrees/agent-*`).

## 8. Verificación (gate del repo)

- [x] 8.1 Desde `packages/app/src-tauri`: `cargo fmt --check` (si falla, `cargo fmt` y re-chequear).
- [x] 8.2 `cargo clippy --all-targets -- -D warnings` sin warnings (CI usa `cargo clippy --all-targets`; el PR template pide `-D warnings`).
- [x] 8.3 `cargo test` — todos verdes, incluidos los tests existentes de `derive_project_name`, backfill v2/v3, v6/v7 y budgets.
- [x] 8.4 Desde la raíz: `pnpm typecheck && pnpm lint && pnpm test:run` (no debería haber cambios de frontend; confirmar que siguen pasando).
- [x] 8.5 (Manual, opcional, read-only) Con una **copia** de `~/Library/Application Support/com.tokenwatch.TokenWatch/tokenwatch.db` (nunca la original), abrir con un binario de prueba o test ignorado que llame `db::open_at(<copia>)` y verificar: `schema_version = 8`; `SELECT COUNT(*) FROM usage_events WHERE project_name='unknown' AND project_path LIKE '%/.paseo/worktrees/%'` == 0; las filas `unknown` restantes son `.claude/worktrees/agent-*`. No commitear ese binario/test.
