## Context

`derive_project_name(cwd)` en `src/ingest/mod.rs` hoy hace: filtra el root y toma los últimos 1–2 componentes (`components[n-2]/components[n-1]`). Contra las rutas de Conductor (`…/conductor/workspaces/<repo>/<ciudad>[/<subpath>]`) eso produce `repo/ciudad` o `ciudad/subpath`, fragmentando el uso.

Muestras reales del `cwd` (verificadas en `~/.claude/projects`):
- `/Users/x/conductor/workspaces/tub2/chengdu-v4`
- `/Users/x/conductor/workspaces/tub2/dili/FRONT/e2e/operator-assignment`
- `/Users/x/conductor/workspaces/argus/cairo/packages/app/src-tauri`
- `/Users/x/conductor/workspaces/tokenwatch/west-monroe-v2`

Los datos ya persistidos (>1 mes) tienen `project_name` incorrecto, pero conservan el `cwd` crudo en `project_path`, así que se pueden recomputar sin re-ingerir.

## Goals / Non-Goals

**Goals:**
- Agrupar el uso por **repo** cuando el `cwd` es un workspace de Conductor.
- Corregir las filas históricas por backfill.
- No romper la derivación para rutas ajenas a Conductor.

**Non-Goals:**
- Distinguir submódulos/sub-proyectos (`FRONT`, `API`, `DAGS`, `packages/…`): por decisión del usuario, **todo se asocia al repo** (`tub2`), no a `tub2/FRONT`.
- Leer `.gitmodules` u otra fuente en disco (el workspace puede ya no existir al ingerir logs viejos).
- Cambiar el contrato del frontend.

## Decisions

**Regla de derivación.** Dado el `cwd` descompuesto en componentes (sin root):
1. Buscar la subsecuencia `conductor` seguida de `workspaces`. Si existe y hay al menos un componente después, `project_name` = **ese componente** (el `<repo>`).
2. Si no hay componente después de `workspaces` → `unknown`.
3. Si no aparece `conductor/workspaces` → **fallback**: comportamiento actual (últimos 1–2 segmentos).
4. `cwd` vacío/ausente → `unknown` (sin cambio).

Detectar por el par literal `conductor/workspaces` (en vez de índice fijo) tolera que el prefijo del home varíe. *Alternativa descartada:* anexar el primer subpath cuando parezca submódulo (heurística mayúsculas / lista configurable) — el usuario tiene submódulos en minúscula y prefiere agrupar todo al repo, así que se elimina esa complejidad.

**Backfill.** Nueva versión de esquema (`schema_version` +1). La migración recorre `usage_events`, recomputa `project_name` desde `project_path` con la nueva regla y actualiza in-place (`UPDATE … SET project_name = ? WHERE dedup_key = ?`, o un `UPDATE` por valor). Idempotente: re-ejecutar sobre datos ya corregidos no cambia nada. Se corre una sola vez, gateada por `schema_version`.

## Risks / Trade-offs

- **Regla literal `conductor/workspaces`** → si alguien tiene un repo cuyo path real contiene esos segmentos por otra razón, se agruparía por el segmento siguiente. *Mitigación:* es el patrón canónico de Conductor y el caso de uso real del usuario; el fallback cubre el resto.
- **Backfill sobre BD grande (~24 MB)** → un `UPDATE` masivo. *Mitigación:* corre una sola vez en una transacción; el volumen es modesto (miles de filas).
- **Fallback no-Conductor sin mejorar** → rutas como `~/dev/inventures/tub2/…` siguen con la heurística vieja. Aceptable: fuera del alcance declarado; se puede iterar después.

## Open Questions

- Ninguna pendiente; la regla y el formato quedaron confirmados por el usuario (asociar todo al repo, sin submódulo).
