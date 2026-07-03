## MODIFIED Requirements

### Requirement: Derivación de nombre de proyecto consciente de Conductor
El sistema SHALL derivar `project_name` reconociendo la estructura de workspaces de Conductor y agrupando los worktrees efímeros de agente. Reglas, en orden:
1. Un `cwd` ausente o vacío MUST resultar en `unknown`.
2. Si el `cwd` contiene cualquier componente `worktrees` (p.ej. `.../.claude/worktrees/agent-XXXX`), el `project_name` MUST ser `unknown`, de modo que todos los worktrees de agente queden agrupados y no se atribuyan a un proyecto.
3. Si el `cwd` contiene los segmentos consecutivos `conductor/workspaces`, el `project_name` MUST ser el segmento inmediatamente posterior a `workspaces` (el repositorio), colapsando el nombre de workspace generado por Conductor, cualquier subpath y cualquier submódulo.
4. En otro caso, el sistema SHALL usar la derivación previa (últimos uno o dos segmentos significativos).

#### Scenario: Worktree efímero de agente agrupado
- **WHEN** el `cwd` es `/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd`
- **THEN** el `project_name` es `unknown`

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
- **WHEN** el `cwd` no contiene `conductor/workspaces` ni un componente `worktrees` (ej. `/Users/x/dev/inventures/tub2`)
- **THEN** el sistema usa la derivación previa (últimos uno o dos segmentos)

#### Scenario: Sin cwd
- **WHEN** el registro no trae `cwd`
- **THEN** el `project_name` es `unknown`

### Requirement: Backfill de nombres de proyecto existentes
El sistema SHALL recomputar `project_name` para las filas ya almacenadas en `usage_events` a partir de su `project_path` (el `cwd` crudo persistido), mediante migraciones gateadas por `schema_version`. Cada migración MUST ser idempotente y no MUST requerir re-ingerir los logs originales. Cuando la regla de derivación cambie, el sistema SHALL introducir una nueva migración de backfill para que las filas históricas adopten la regla vigente.

#### Scenario: Worktrees históricos corregidos
- **WHEN** existen filas cuyo `project_path` es un worktree de agente y cuyo `project_name` fue derivado por una regla antigua (ej. `worktrees/agent-XXXX`) y se aplica la migración vigente
- **THEN** esas filas quedan con `project_name = unknown`, sin alterar su `project_path`

#### Scenario: Proyectos de Conductor preservados
- **WHEN** existen filas de workspaces de Conductor (ej. `project_path = /Users/x/conductor/workspaces/tub2/chengdu-v4`) y se aplica la migración vigente
- **THEN** esas filas quedan con `project_name` igual al repositorio (ej. `tub2`)

#### Scenario: Migración idempotente
- **WHEN** una migración de backfill se ejecuta sobre una base cuyo `schema_version` ya la incluye
- **THEN** no se realizan cambios adicionales
