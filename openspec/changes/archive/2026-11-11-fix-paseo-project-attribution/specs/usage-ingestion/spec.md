## ADDED Requirements

### Requirement: Resolución de proyecto para worktrees de Paseo
Cuando el `cwd` contiene los componentes consecutivos `.paseo` → `worktrees` seguidos de `<id>` y `<slug>` (layout `~/.paseo/worktrees/<id>/<slug>[/<subpath>]`), el sistema SHALL atribuir el evento al **repositorio principal** del worktree, aplicando esta cadena en orden:
1. Índice construido a partir de `~/.paseo/projects/workspaces.json` (array de entradas con `worktreeRoot`, `cwd` y `mainRepoRoot`, incluyendo worktrees archivados). El sistema SHALL buscar la entrada por el prefijo de ruta más largo (en límites de componente) del `cwd`, de modo que los subdirectorios del worktree también resuelvan. El `project_name` MUST derivarse de `mainRepoRoot` aplicando las reglas no-Paseo de derivación (p. ej. `/Users/x/dev/freelance/tokenwatch` → `freelance/tokenwatch`).
2. Si no hay entrada en el índice: el archivo `.git` del worktree root (`gitdir: <repo>/.git/worktrees/<x>`); el repo principal es la ruta anterior a `/.git/`, y el nombre se deriva igual que en el paso 1.
3. Si ninguno resuelve: el `project_name` MUST ser el segmento `<slug>`, nunca `unknown`.

El índice MUST cargarse una vez por corrida de ingesta (y una vez por backfill) y pasarse a la derivación, que MUST permanecer libre de E/S. Un `workspaces.json` ausente, ilegible, malformado o con formato inesperado MUST tratarse como índice vacío (con log) y MUST NOT provocar panic ni abortar la ingesta. El sistema MUST NOT usar el segmento `<id>` para derivar el nombre.

#### Scenario: Worktree de Paseo presente en el índice
- **WHEN** el `cwd` es `/Users/x/.paseo/worktrees/3qkyl933/resolute-seal` y el índice mapea ese `worktreeRoot` a `mainRepoRoot = /Users/x/dev/freelance/tokenwatch`
- **THEN** el `project_name` es `freelance/tokenwatch`

#### Scenario: Subdirectorio de un worktree de Paseo
- **WHEN** el `cwd` es `/Users/x/.paseo/worktrees/abcd1234/upbeat-bear/dev-wiki` y el índice mapea `/Users/x/.paseo/worktrees/abcd1234/upbeat-bear` a `/Users/x/dev/meki/backend`
- **THEN** el `project_name` es `meki/backend`

#### Scenario: Worktree archivado sigue resolviendo
- **WHEN** la entrada del índice tiene `archivedAt` no nulo
- **THEN** se usa igual para resolver el repo principal

#### Scenario: Fallback por archivo .git
- **WHEN** el `cwd` es un worktree de Paseo ausente del índice cuyo archivo `.git` contiene `gitdir: /Users/x/dev/meki/backend/.git/worktrees/upbeat-bear`
- **THEN** el `project_name` es `meki/backend`

#### Scenario: Último recurso: slug
- **WHEN** el `cwd` es `/Users/x/.paseo/worktrees/zz99/brave-otter` y no está en el índice ni tiene un `.git` legible
- **THEN** el `project_name` es `brave-otter`

#### Scenario: Ruta Paseo incompleta
- **WHEN** el `cwd` termina en `.paseo/worktrees` o en `.paseo/worktrees/<id>` (sin slug)
- **THEN** el `project_name` es `unknown`

#### Scenario: workspaces.json malformado
- **WHEN** `~/.paseo/projects/workspaces.json` contiene JSON inválido o un objeto en lugar de un array
- **THEN** el sistema usa un índice vacío, registra un log, no hace panic y continúa con los fallbacks

#### Scenario: workspaces.json ausente
- **WHEN** no existe `~/.paseo/projects/workspaces.json`
- **THEN** la ingesta continúa normalmente usando los fallbacks

## MODIFIED Requirements

### Requirement: Derivación de nombre de proyecto consciente de Conductor
El sistema SHALL derivar `project_name` reconociendo la estructura de workspaces de Conductor y de worktrees de Paseo, y agrupando los worktrees efímeros de agente. Reglas, en orden:
1. Un `cwd` ausente o vacío MUST resultar en `unknown`.
2. Si el `cwd` contiene los componentes consecutivos `.paseo` → `worktrees`, el `project_name` MUST resolverse según el requirement "Resolución de proyecto para worktrees de Paseo".
3. Si el `cwd` contiene el par consecutivo `.claude` → `worktrees` (p.ej. `.../.claude/worktrees/agent-XXXX`), o un componente `worktrees` seguido inmediatamente de un componente que empieza con `agent-`, el `project_name` MUST ser `unknown`, de modo que los worktrees efímeros de agente queden agrupados y no se atribuyan a un proyecto. Un componente `worktrees` fuera de estos casos MUST NOT por sí solo producir `unknown`.
4. Si el `cwd` contiene los segmentos consecutivos `conductor/workspaces`, el `project_name` MUST ser el segmento inmediatamente posterior a `workspaces` (el repositorio), colapsando el nombre de workspace generado por Conductor, cualquier subpath y cualquier submódulo.
5. En otro caso, el sistema SHALL usar la derivación previa (últimos uno o dos segmentos significativos).

#### Scenario: Worktree efímero de agente agrupado
- **WHEN** el `cwd` es `/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd`
- **THEN** el `project_name` es `unknown`

#### Scenario: Worktree de agente dentro de un workspace de Conductor
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/tub2/kelowna-v1/FRONT/.claude/worktrees/agent-ad31`
- **THEN** el `project_name` es `unknown`

#### Scenario: Worktree de agente sin .claude
- **WHEN** el `cwd` es `/Users/x/worktrees/agent-1234/packages/app`
- **THEN** el `project_name` es `unknown`

#### Scenario: Worktree de Paseo no cae en unknown
- **WHEN** el `cwd` es `/Users/x/.paseo/worktrees/3qkyl933/resolute-seal`
- **THEN** el `project_name` NO es `unknown` y se resuelve según la cadena de Paseo

#### Scenario: Componente worktrees genérico
- **WHEN** el `cwd` es `/Users/x/dev/worktrees/feature-x` (sin `.claude` previo ni `agent-` posterior)
- **THEN** el sistema usa la derivación previa (`worktrees/feature-x`)

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
- **WHEN** el `cwd` no contiene `conductor/workspaces`, `.paseo/worktrees` ni un worktree de agente (ej. `/Users/x/dev/inventures/tub2`)
- **THEN** el sistema usa la derivación previa (últimos uno o dos segmentos)

#### Scenario: Sin cwd
- **WHEN** el registro no trae `cwd`
- **THEN** el `project_name` es `unknown`

### Requirement: Backfill de nombres de proyecto existentes
El sistema SHALL recomputar `project_name` para las filas ya almacenadas en `usage_events` a partir de su `project_path` (el `cwd` crudo persistido), mediante migraciones gateadas por `schema_version`. Cada migración MUST ser idempotente y no MUST requerir re-ingerir los logs originales. Cuando la regla de derivación cambie, el sistema SHALL introducir una nueva migración de backfill para que las filas históricas adopten la regla vigente. El backfill vigente MUST usar el índice de Paseo cargado una vez al inicio de la migración y SHALL escribir sólo las filas cuyo `project_name` cambia. Un fallo al cargar el índice de Paseo MUST NOT hacer fallar la migración.

#### Scenario: Worktrees históricos corregidos
- **WHEN** existen filas cuyo `project_path` es un worktree de agente y cuyo `project_name` fue derivado por una regla antigua (ej. `worktrees/agent-XXXX`) y se aplica la migración vigente
- **THEN** esas filas quedan con `project_name = unknown`, sin alterar su `project_path`

#### Scenario: Filas de Paseo re-atribuidas
- **WHEN** existen filas con `project_name = unknown` cuyo `project_path` es un worktree de Paseo resoluble (ej. `/Users/x/.paseo/worktrees/3qkyl933/resolute-seal` → `mainRepoRoot = /Users/x/dev/freelance/tokenwatch`) y se aplica la migración vigente (v8)
- **THEN** esas filas quedan con `project_name = freelance/tokenwatch`, sin alterar su `project_path`

#### Scenario: Proyectos de Conductor preservados
- **WHEN** existen filas de workspaces de Conductor (ej. `project_path = /Users/x/conductor/workspaces/tub2/chengdu-v4`) y se aplica la migración vigente
- **THEN** esas filas quedan con `project_name` igual al repositorio (ej. `tub2`)

#### Scenario: Migración idempotente
- **WHEN** una migración de backfill se ejecuta sobre una base cuyo `schema_version` ya la incluye
- **THEN** no se realizan cambios adicionales

#### Scenario: Versión de esquema tras la migración
- **WHEN** se abre una base nueva o existente con la versión vigente del binario
- **THEN** `schema_version` queda en `8`
