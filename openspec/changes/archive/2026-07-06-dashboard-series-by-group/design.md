## Context

El comando `query_series` (`packages/app/src-tauri/src/usage/mod.rs`) construye dinámicamente una expresión SQL para el nombre de serie a partir del enum `SeriesBy` y hace `GROUP BY bucket_label, series_name` directamente en SQLite:

```rust
let series_col = match params.series_by {
    SeriesBy::Model => "model".to_owned(),
    SeriesBy::Project => "project_name".to_owned(),
    SeriesBy::ModelProject => "model || ' · ' || project_name".to_owned(),
};
```

El `FROM` es siempre `usage_events`, y el `WHERE` opcional solo referencia `timestamp`. La lista completa de buckets se calcula aparte con `all_buckets_in_range` (que opera solo sobre `usage_events`), y el ensamblado de series (0-fill) es genérico respecto al nombre de serie.

Los grupos ya existen (PR #17) en `budgets/mod.rs`: tablas `project_groups(id, name, ...)` y `project_group_members(project_name, group_id)` con `ON DELETE CASCADE` (al borrar un grupo, sus miembros vuelven a "otros"). El módulo de budgets ya usa la convención de colapsar proyectos sin grupo en una serie llamada `"otros"`.

El frontend expone el selector de series en `ChartControls.tsx` a partir de `SERIES_BY_OPTIONS`, tipado por `SeriesBy` en `types.ts`. El gráfico, la tabla y los colores derivan del array `series` de la respuesta, indexados por `name`, así que no requieren cambios estructurales para una nueva dimensión.

## Goals / Non-Goals

**Goals:**
- Añadir `Group` al enum `SeriesBy` (backend) y `"group"` al tipo `SeriesBy` (frontend).
- Agregar el uso por grupo de proyecto en `query_series`, reutilizando `project_group_members` / `project_groups`.
- Colapsar proyectos sin grupo en una única serie `"otros"`.
- Reflejar la selección en el selector "Series" del dashboard; gráfico y tabla la consumen sin cambios adicionales.
- Cubrir con tests Rust: agregación por grupo, "otros", y proyecto reasignado.

**Non-Goals:**
- `modelGroup` (cruce modelo × grupo) — posible follow-up, fuera de alcance.
- Cambiar la paleta de colores o el orden de series (se reutiliza el mecanismo existente por `name`).
- Migraciones de esquema — las tablas de grupos ya existen desde PR #17.
- Cambiar `query_today_by_project` / el popover.

## Decisions

### D1: Mapear grupo en SQL con LEFT JOIN + COALESCE (no post-proceso en Rust)

Cuando `series_by = Group`, extender el `FROM` con:

```sql
FROM usage_events e
LEFT JOIN project_group_members m ON m.project_name = e.project_name
LEFT JOIN project_groups g ON g.id = m.group_id
```

y usar como `series_col`:

```sql
COALESCE(g.name, 'otros')
```

**Por qué:** mantiene toda la agregación (GROUP BY, SUM, 0-fill) en el mismo camino de código ya probado. El resto de `query_series_inner` (buckets, value_map, ensamblado) queda intacto porque solo depende de `series_name`. Alternativa considerada: traer filas por `project_name` y remapear en Rust con un HashMap (como hace `budgets`); se descarta porque duplicaría la lógica de bucketing/0-fill y agregación fuera de SQL sin beneficio.

**Cuidado con la ambigüedad de columnas:** al introducir los JOINs, `project_name` existe en `usage_events` y en `project_group_members`. Por eso, para la rama `Group` se debe:
- Alias `usage_events AS e` y cualificar `e.project_name` / `e.model` / `e.timestamp` / `e.total_tokens` / `e.cost` donde corresponda.
- El resto de ramas (`Model`, `Project`, `ModelProject`) siguen sin JOIN sobre `usage_events` sin alias, como hoy.

Para evitar tocar el `WHERE`/`metric_expr` compartidos, la opción más limpia es hacer que **el alias `e` esté disponible solo cuando hay JOIN**. Se puede lograr construyendo la cláusula `FROM` condicionalmente y usando siempre columnas cualificadas con un prefijo variable (p. ej. `let tbl = "usage_events"` sin alias por defecto, y `let tbl = "usage_events e"` + prefijo `e.` para la rama Group). Ver D2 para la estrategia concreta de refactor mínimo.

### D2: Refactor mínimo del builder SQL

Introducir un pequeño branch en `query_series_inner`:

- Calcular `from_clause` y un prefijo de columna (`col_prefix`) según `series_by`:
  - Ramas actuales: `from_clause = "usage_events"`, `col_prefix = ""`.
  - `Group`: `from_clause = "usage_events e LEFT JOIN project_group_members m ON m.project_name = e.project_name LEFT JOIN project_groups g ON g.id = m.group_id"`, `col_prefix = "e."`.
- `series_col` para `Group` = `"COALESCE(g.name, 'otros')"`.
- Usar `{col_prefix}total_tokens` / `{col_prefix}cost` en `metric_expr` y `{col_prefix}timestamp` en el `WHERE` y el `strftime`. Como `col_prefix` es `""` en las ramas existentes, el SQL generado para ellas es idéntico byte a byte (sin regresión).
- `all_buckets_in_range` no cambia: sigue operando sobre `usage_events` con el mismo `where_clause` (que ahora usa `timestamp` sin prefijo — mantener esa función independiente del alias). Verificar que el `where_clause` que consume esa función siga usando `timestamp` sin cualificar; si se decide cualificar el WHERE para la query principal, pasar una variante sin prefijo a `all_buckets_in_range`.

**Nota de implementación:** la ruta más segura es mantener el `where_clause` de `all_buckets_in_range` exactamente como está (columna `timestamp` desnuda sobre `usage_events`) y, para la query principal con JOIN, o bien (a) cualificar solo si `col_prefix != ""`, o (b) apoyarse en que SQLite resuelve `timestamp` sin ambigüedad porque solo `usage_events` la tiene. `timestamp`, `total_tokens`, `cost` y `model` existen únicamente en `usage_events`, así que **solo `project_name` es ambigua** — basta cualificar `e.project_name` en la condición del JOIN y usar `COALESCE(g.name,'otros')` en el SELECT; las demás columnas pueden ir sin prefijo. Esto minimiza el refactor: no hace falta `col_prefix` si se acepta que las columnas no ambiguas queden sin cualificar.

Decisión final: **no introducir `col_prefix`; solo cambiar `from_clause` y `series_col` para la rama `Group`, cualificando `e.project_name` únicamente en el ON del JOIN.** Es el diff más pequeño y sin riesgo de ambigüedad.

### D3: Nombre de la serie sin grupo = "otros"

Reutilizar el literal `'otros'` (mismo string que `budgets/mod.rs`). Esto garantiza consistencia visual con el popover/budgets. No se traduce ni se hace configurable.

### D4: Frontend — extender tipo y selector

- `types.ts`: `export type SeriesBy = "model" | "project" | "modelProject" | "group";`
- `ChartControls.tsx`: añadir `{ value: "group", label: "Grupo" }` a `SERIES_BY_OPTIONS`.
- El resto (colores en `colors.ts`, `UsageChart`, `UsageTable`, `useUsageSeries`) consume `series[].name` genéricamente; no requiere cambios. La serialización `serde(rename_all = "camelCase")` mapea `"group"` ⇄ `SeriesBy::Group` automáticamente.

## Risks / Trade-offs

- **[Ambigüedad de columna `project_name` al añadir JOINs]** → Cualificar `e.project_name` en la condición del JOIN; las demás columnas del `SELECT`/`WHERE` son exclusivas de `usage_events` y no requieren alias. Cubierto por tests de agregación por grupo.
- **[Fila duplicada por evento si `project_group_members` tuviera múltiples filas por proyecto]** → El esquema tiene `project_name` como clave única (UPSERT `ON CONFLICT(project_name)`), así que el LEFT JOIN produce a lo sumo una fila por evento. Riesgo nulo mientras se mantenga esa unicidad.
- **[Deriva de `project_name`]** → `project_name` es un string derivado; una futura re-derivación en migración debe remapear membresías (ya documentado en `backfill_project_names`). Esta feature no cambia esa situación, solo la consume.
- **[Regresión en ramas existentes de `series_by`]** → El SQL de `Model`/`Project`/`ModelProject` se mantiene idéntico (sin JOIN, sin alias). Los tests existentes de `query_series_inner` deben seguir pasando sin cambios.
- **[Sin grupos definidos]** → Con series=grupo y ninguna membresía, todo cae en "otros" (una sola serie). Comportamiento correcto y esperado; añadir un test.
