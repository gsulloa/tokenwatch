## Why

Desde ~2026-10-12..16 casi todo el uso registrado por TokenWatch aparece bajo el proyecto `unknown`: los totales de tokens son correctos, pero la atribución por proyecto está rota. La causa es la regla de `derive_project_name` (`packages/app/src-tauri/src/ingest/mod.rs`) que devuelve `unknown` ante **cualquier** componente `worktrees` en el `cwd`. Esa regla se pensó para los worktrees efímeros de agente de Claude (`.claude/worktrees/agent-XXXX`), pero el usuario migró de Conductor a **Paseo**, que ubica cada checkout en `~/.paseo/worktrees/<id>/<slug>` (p. ej. `/Users/x/.paseo/worktrees/3qkyl933/resolute-seal`). Toda sesión de Paseo cae por lo tanto en `unknown`.

Evidencia en la base local (`tokenwatch.db`, `schema_version = 7`): ~120.009 filas `unknown` (~15,19 B tokens), de las cuales ~112.767 (~14,64 B tokens) son rutas de Paseo; ~7.182 son `.claude/worktrees/agent-*` reales y ~60 son worktrees de agente dentro de workspaces de Conductor (también `.../.claude/worktrees/agent-*`). La ingesta incremental nunca relee líneas viejas, así que sin un backfill los datos históricos seguirían mal atribuidos.

## What Changes

- **Resolución de worktrees de Paseo**: cuando el `cwd` está bajo `.paseo/worktrees/<id>/<slug>[/...]`, el `project_name` se deriva del **repositorio principal** (mismo criterio de producto que Conductor: el uso se agrupa por repo real), con esta cadena de resolución:
  1. Índice de `~/.paseo/projects/workspaces.json` (archivo interno de Paseo; mapea `worktreeRoot`/`cwd` → `mainRepoRoot`, incluye worktrees archivados), con match por prefijo de ruta más largo para cubrir subdirectorios. El nombre se obtiene de `mainRepoRoot` con la regla existente de últimos dos segmentos (p. ej. `freelance/tokenwatch`, `meki/backend`).
  2. Fallback: el archivo `.git` del worktree (`gitdir: <repo>/.git/worktrees/<x>`) → ruta anterior a `/.git/`.
  3. Último recurso: el segmento `<slug>` en lugar de `unknown`.
  Cualquier error (archivo ausente, JSON malformado, formato cambiado) cae al siguiente paso; nunca hay panic.
- **Regla de worktrees efímeros acotada**: `unknown` sólo cuando el `cwd` contiene el par adyacente `.claude` → `worktrees`, o un componente `worktrees` seguido de un componente `agent-*`. Los worktrees de agente dentro de Conductor siguen en `unknown` (sin cambio de comportamiento).
- **`derive_project_name` sigue siendo pura y testeable**: el índice de Paseo se carga una vez por corrida de ingesta (y una vez por backfill) y se pasa como parámetro (`PaseoIndex`). **BREAKING (API interna Rust)**: cambia la firma de `derive_project_name` e `ingest_from`.
- **Migración v8**: re-ejecuta `backfill_project_names` con el resolver nuevo para re-atribuir las ~112k filas `unknown` ya almacenadas, sin re-ingerir JSONL.

## Capabilities

### New Capabilities
<!-- Ninguna capability nueva: la resolución de Paseo es parte de la ingesta de uso. -->

### Modified Capabilities
- `usage-ingestion`: se modifica el requirement "Derivación de nombre de proyecto consciente de Conductor" (regla de worktrees acotada + regla de Paseo antes de ella), se agrega el requirement "Resolución de proyecto para worktrees de Paseo" y se modifica "Backfill de nombres de proyecto existentes" para cubrir la re-atribución de filas de Paseo.

## Impact

- **Backend (`packages/app/src-tauri`)**:
  - `src/ingest/mod.rs`: nuevo `PaseoIndex` (carga de `workspaces.json`, fallback `.git`), cambio de firma de `derive_project_name(cwd, &PaseoIndex)`, `ingest_from(conn, base_dir, &PaseoIndex)`, `ingest_file(...)`; tests nuevos y actualizados.
  - `src/db/mod.rs`: `backfill_project_names(conn, &PaseoIndex)`, nueva `apply_v8`, `migrate` sube a `schema_version = 8`; tests de versión actualizados.
  - `src/budgets/mod.rs`: assert de `schema_version` en tests pasa de `"7"` a `"8"`.
- **Datos**: la migración v8 reescribe `project_name` en filas existentes (principalmente `unknown` → repo real); `project_path` no cambia. `project_group_members` no se remapea (ver design).
- **Dependencias**: ninguna nueva (`serde_json` ya está en el crate).
- **Frontend**: sin cambios de contrato; las series por proyecto quedan correctamente agrupadas.
- **Dependencia externa no documentada**: el formato de `~/.paseo/projects/workspaces.json` es interno de Paseo; se mitiga con la cadena de fallbacks.
