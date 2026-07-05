# usage-ingestion Specification

## Purpose
TBD - created by archiving change usage-charts. Update Purpose after archive.
## Requirements
### Requirement: Parseo nativo de logs JSONL de Claude
El sistema SHALL leer los archivos `~/.claude/projects/**/*.jsonl` y extraer, de cada registro `type: "assistant"` que contenga `message.usage`, los campos: `message.id`, `requestId`, `sessionId`, `cwd`, `timestamp`, `message.model` y los cuatro tipos de token (input, output, cache_creation, cache_read). El parseo MUST ser nativo en Rust, sin depender de `ccusage` ni de `node` en runtime.

#### Scenario: Registro assistant válido
- **WHEN** el sistema procesa una línea `type: "assistant"` con `message.usage`
- **THEN** produce un evento de uso con los cuatro conteos de token, modelo, proyecto, sesión y timestamp

#### Scenario: Línea sin usage se ignora
- **WHEN** el sistema encuentra una línea sin `message.usage` (p.ej. `type: "user"` o metadatos)
- **THEN** la salta sin error y continúa con la siguiente

#### Scenario: Campos desconocidos toleran cambios de formato
- **WHEN** una línea trae campos adicionales o desconocidos
- **THEN** el sistema los ignora y parsea igual los campos requeridos

### Requirement: Deduplicación de eventos
El sistema SHALL deduplicar los eventos usando la clave `"{message.id}:{requestId}"`, de modo que un mismo mensaje que aparezca en sesiones reanudadas o sidechains se cuente una sola vez.

#### Scenario: Mensaje duplicado
- **WHEN** el mismo `message.id` + `requestId` se procesa más de una vez
- **THEN** solo existe una fila en la base de datos para esa clave

### Requirement: Persistencia local en SQLite que sobrevive al borrado del log
El sistema SHALL almacenar cada evento como una fila en una base SQLite local (grano por-mensaje), preservando los datos aunque el archivo JSONL original se borre o se rote.

#### Scenario: Historia preservada tras borrado del JSONL
- **WHEN** un archivo JSONL ya ingerido se elimina del disco
- **THEN** sus eventos permanecen consultables en SQLite

#### Scenario: Ingesta idempotente
- **WHEN** se re-ejecuta la ingesta sobre datos ya procesados
- **THEN** no se crean filas duplicadas

### Requirement: Derivación del nombre de proyecto
El sistema SHALL derivar un `project_name` legible desde `cwd` y almacenar también el `cwd` crudo (`project_path`). Los registros sin `cwd` MUST asignarse a `unknown`.

#### Scenario: Nombre derivado de cwd
- **WHEN** un evento trae `cwd = "/Users/x/conductor/workspaces/backend/madrid"`
- **THEN** su `project_name` es un nombre legible derivado (p.ej. `backend/madrid`)

#### Scenario: Sin cwd
- **WHEN** un evento no trae `cwd`
- **THEN** su `project_name` es `unknown`

### Requirement: Cálculo de costo estimado
El sistema SHALL calcular un costo estimado por evento como `input·Pin + output·Pout + cache_creation·Pwrite + cache_read·Pread`, usando una tabla de precios embebida por modelo. Un modelo ausente de la tabla MUST resultar en costo 0 y registrar un log, sin interrumpir la ingesta.

#### Scenario: Modelo conocido
- **WHEN** el evento usa un modelo presente en la tabla de precios
- **THEN** el costo se calcula con las tarifas de ese modelo

#### Scenario: Modelo desconocido
- **WHEN** el evento usa un modelo ausente de la tabla
- **THEN** el costo es 0, se registra un log y la ingesta continúa

### Requirement: Ingesta incremental y polling
El sistema SHALL ingerir de forma incremental (reparseando solo las líneas nuevas de cada archivo según `size`/`mtime` y un offset de líneas ya procesadas) y SHALL ejecutar un refresh periódico en segundo plano que, al terminar, emita el evento Tauri `usage-updated`.

#### Scenario: Archivo sin cambios se salta
- **WHEN** un archivo tiene el mismo `size` y `mtime` que en la última ingesta
- **THEN** el sistema no lo reparsea

#### Scenario: Archivo con líneas nuevas
- **WHEN** un archivo creció desde la última ingesta
- **THEN** el sistema parsea solo las líneas nuevas y actualiza el offset

#### Scenario: Notificación al frontend
- **WHEN** un ciclo de refresh termina con datos nuevos
- **THEN** el sistema emite el evento `usage-updated`

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

