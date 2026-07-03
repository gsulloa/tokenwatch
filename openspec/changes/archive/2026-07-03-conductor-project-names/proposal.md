## Why

La derivación actual de `project_name` toma los últimos 1–2 segmentos del `cwd`, lo que produce nombres incorrectos para los workspaces de Conductor. Conductor crea rutas `…/conductor/workspaces/<repo>/<ciudad-workspace>[/<subpath>]`, donde `<ciudad-workspace>` es un nombre generado (ej. `cairo`, `chengdu-v4`, `belo-horizonte-v1`, con sufijo opcional `-vN`). Resultado: el uso queda fragmentado por workspace efímero en vez de agruparse por proyecto real.

Ejemplos del bug (salida actual → deseada):
- `tub2/chengdu-v4` → **`tub2`**
- `argus/cairo` → **`argus`**
- `tub2/dili/FRONT` → **`tub2`** (submódulos y subpaths se asocian al repo)
- `argus/cairo/packages/app/src-tauri` → **`argus`**

## What Changes

- Derivación de `project_name` **consciente de Conductor**: cuando el `cwd` contiene `conductor/workspaces/`, el proyecto es el **segmento inmediatamente posterior** (`<repo>`), colapsando el nombre de ciudad, cualquier subpath y cualquier submódulo hacia ese repo.
- Rutas que **no** son de Conductor conservan la derivación actual (sin regresión).
- **Backfill** de las filas ya almacenadas: recomputar `project_name` desde `project_path` (el `cwd` crudo ya persistido), para corregir los datos históricos sin re-ingerir.

## Capabilities

### New Capabilities
<!-- Ninguna capability nueva. -->

### Modified Capabilities
- `usage-ingestion`: se refina el requirement de derivación de `project_name` para reconocer la estructura de workspaces de Conductor y agrupar por repo; se añade backfill de filas existentes.

## Impact

- **Backend (`packages/app/src-tauri`):** `src/ingest/mod.rs` (`derive_project_name`) y una migración de backfill en `src/db/mod.rs` (recomputar `project_name` sobre `usage_events`, keyed por `schema_version`).
- **Datos:** la migración reescribe `project_name` en filas existentes; `project_path` (cwd crudo) no cambia.
- **Frontend:** sin cambios de contrato; las series por proyecto simplemente quedan bien agrupadas.
