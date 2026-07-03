## ADDED Requirements

### Requirement: Derivación de nombre de proyecto consciente de Conductor
El sistema SHALL derivar `project_name` reconociendo la estructura de workspaces de Conductor. Cuando el `cwd` contiene los segmentos consecutivos `conductor/workspaces`, el `project_name` MUST ser el segmento inmediatamente posterior a `workspaces` (el repositorio), colapsando el nombre de workspace generado por Conductor, cualquier subpath y cualquier submódulo. Cuando el `cwd` no corresponde a un workspace de Conductor, el sistema SHALL conservar la derivación previa (últimos uno o dos segmentos significativos). Un `cwd` ausente o vacío MUST resultar en `unknown`.

#### Scenario: Workspace root de Conductor
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/tub2/chengdu-v4`
- **THEN** el `project_name` es `tub2`

#### Scenario: Ciudad con sufijo de versión
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/argus/cairo` o `.../argus/belo-horizonte-v1`
- **THEN** el `project_name` es `argus`

#### Scenario: Submódulo dentro del workspace
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/tub2/dili/FRONT/e2e/operator-assignment`
- **THEN** el `project_name` es `tub2`

#### Scenario: Subpath profundo de monorepo
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/argus/cairo/packages/app/src-tauri`
- **THEN** el `project_name` es `argus`

#### Scenario: Ruta ajena a Conductor
- **WHEN** el `cwd` no contiene `conductor/workspaces` (ej. `/Users/x/dev/inventures/tub2`)
- **THEN** el sistema usa la derivación previa (últimos uno o dos segmentos)

#### Scenario: Sin cwd
- **WHEN** el registro no trae `cwd`
- **THEN** el `project_name` es `unknown`

### Requirement: Backfill de nombres de proyecto existentes
El sistema SHALL recomputar `project_name` para las filas ya almacenadas en `usage_events` a partir de su `project_path` (el `cwd` crudo persistido), mediante una migración gateada por `schema_version` que se ejecuta una sola vez. La migración MUST ser idempotente y no MUST requerir re-ingerir los logs originales.

#### Scenario: Filas históricas corregidas
- **WHEN** existen filas con `project_name` derivado por la regla antigua (ej. `tub2/chengdu-v4`) y se aplica la migración
- **THEN** esas filas quedan con el `project_name` nuevo (ej. `tub2`), sin alterar su `project_path`

#### Scenario: Migración idempotente
- **WHEN** la migración se ejecuta sobre una base cuyo `schema_version` ya está actualizado
- **THEN** no se realizan cambios adicionales
