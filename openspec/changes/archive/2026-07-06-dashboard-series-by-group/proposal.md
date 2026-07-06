## Why

El dashboard ya segmenta el uso por `model`, `project` y `modelProject`, pero con la llegada de los grupos de proyecto (PR #17) no hay forma de ver las series agregadas por grupo. Quien organiza sus proyectos en grupos (p. ej. cliente, equipo o área) no puede leer el consumo agregado por esa dimensión en el gráfico ni en la tabla de detalle.

## What Changes

- Añadir `group` como nueva opción de `seriesBy` en la consulta de series (`query_series`), agregando el uso por grupo de proyecto en lugar de por proyecto o modelo individual.
- En la agregación, mapear cada `project_name` a su grupo usando `project_group_members` → `project_groups`; los proyectos sin grupo se colapsan en la serie **"otros"** (consistente con el popover y `project-budgets`).
- Añadir la opción "Grupo" al selector de Series en `ChartControls`; los colores y labels de las series usan el nombre del grupo.
- La tabla de detalle y las tarjetas de resumen reflejan automáticamente la nueva segmentación (comparten la misma respuesta de `query_series`).
- Fuera de alcance (posible follow-up): `modelGroup` (cruce modelo × grupo).

## Capabilities

### New Capabilities
<!-- Ninguna. -->

### Modified Capabilities
- `usage-charts`: la consulta de series agregadas admite una nueva dimensión de segmentación por grupo de proyecto, con proyectos sin grupo colapsados en "otros".

## Impact

- **Backend** (`packages/app/src-tauri/src/usage/mod.rs`): extender el enum `SeriesBy` con `Group`; en `query_series_inner`, mapear `project_name` → nombre de grupo (join contra `project_group_members`/`project_groups`) con `COALESCE(..., 'otros')`.
- **Frontend** (`packages/app/src/features/usage/types.ts`, `ChartControls.tsx`): extender el tipo `SeriesBy` y agregar la opción "Grupo" al selector.
- **Datos**: depende de las tablas `project_groups` / `project_group_members` (PR #17). Reutiliza la clave `project_name`; ver la nota de re-derivación en `backfill_project_names`.
- **Tests**: nuevos casos de agregación por grupo en Rust (incluye "otros" y proyectos reasignados) y actualización de `ChartControls.test.tsx`.
