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
- **THEN** su `project_name` es un nombre legible derivado del repositorio principal (p.ej. `meki/backend` si el workspace de Conductor resuelve a `/Users/x/dev/meki/backend`)

#### Scenario: Sin cwd
- **WHEN** un evento no trae `cwd`
- **THEN** su `project_name` es `unknown`

### Requirement: Cálculo de costo estimado
El sistema SHALL calcular un costo estimado por evento como `input·Pin + output·Pout + cache_creation·Pwrite + cache_read·Pread`, usando una tabla de precios embebida por **familia de modelo**. La tabla SHALL cubrir, como mínimo, las familias que Claude Code emite hoy: el tier premium (`fable` / `mythos`), `opus`, `sonnet` y `haiku`.

El emparejamiento de un id de modelo a su fila de precios SHALL resolver por el nombre de familia y MUST NOT depender de la versión mayor del modelo, de modo que nuevas versiones mayores de una familia conocida (p. ej. `claude-sonnet-5`, `claude-opus-5`) se contabilicen con la fila de precios de esa familia. El emparejamiento MUST tolerar alias desnudos (`sonnet`, `opus`) y prefijos de proveedor (p. ej. `global.anthropic.claude-sonnet-4-6`).

Las familias `fable` y `mythos` SHALL compartir una única fila de precios, dado que Claude Fable 5 y Claude Mythos 5 tienen precios idénticos y difieren sólo en el canal de distribución. Esa fila SHALL ser más cara que la de `opus`, reflejando que el tier premium se tarifa por encima del tier Opus.

Un modelo cuya familia no esté en la tabla MUST resultar en costo 0 y registrar un log, sin interrumpir la ingesta. Una suite de tests SHALL verificar que toda familia que Claude Code emite resuelve a una fila de precios, de modo que la ausencia de una familia falle en CI en lugar de producir silenciosamente costo 0.

#### Scenario: Modelo de familia conocida, cualquier versión
- **WHEN** el evento usa un modelo cuya familia está en la tabla, en cualquier versión mayor (p. ej. `claude-sonnet-5`, `claude-opus-4-8`, `claude-haiku-4-5`)
- **THEN** el costo se calcula con las tarifas de esa familia

#### Scenario: Modelo del tier premium
- **WHEN** el evento usa `claude-fable-5`
- **THEN** el costo se calcula con la fila del tier premium, cuyas tarifas de input y output son mayores que las de la familia `opus`, y el costo resultante es mayor que cero

#### Scenario: Mythos comparte la fila de Fable
- **WHEN** el evento usa `claude-mythos-5` o `claude-mythos-preview`
- **THEN** el costo se calcula con exactamente la misma fila de precios que `claude-fable-5`

#### Scenario: Alias desnudo o prefijo de proveedor
- **WHEN** el evento usa un id sin versión (p. ej. `sonnet`) o con prefijo de proveedor (p. ej. `global.anthropic.claude-sonnet-4-6`)
- **THEN** el costo se calcula con las tarifas de la familia contenida en el id

#### Scenario: Modelo de familia desconocida
- **WHEN** el evento usa un modelo cuya familia no está en la tabla (p. ej. `gpt-4o`)
- **THEN** el costo es 0, se registra un log y la ingesta continúa

#### Scenario: Familia faltante detectada en CI
- **WHEN** una familia que Claude Code emite no tiene fila en la tabla embebida
- **THEN** la suite de tests falla, en lugar de que los eventos de esa familia se registren silenciosamente con costo 0

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
El sistema SHALL derivar `project_name` reconociendo la estructura de workspaces de Conductor y de worktrees de Paseo, y agrupando los worktrees efímeros de agente. Reglas, en orden:
1. Un `cwd` ausente o vacío MUST resultar en `unknown`.
2. Si el `cwd` contiene los componentes consecutivos `.paseo` → `worktrees`, el `project_name` MUST resolverse según el requirement "Resolución de proyecto para worktrees de Paseo".
3. Si el `cwd` contiene el par consecutivo `.claude` → `worktrees` (p.ej. `.../.claude/worktrees/agent-XXXX`), o un componente `worktrees` seguido inmediatamente de un componente que empieza con `agent-`, el `project_name` MUST ser `unknown`, de modo que los worktrees efímeros de agente queden agrupados y no se atribuyan a un proyecto. Un componente `worktrees` fuera de estos casos MUST NOT por sí solo producir `unknown`.
4. Si el `cwd` contiene los segmentos consecutivos `conductor/workspaces`, el `project_name` MUST resolverse según el requirement "Resolución de proyecto para workspaces de Conductor" (nombre de dos partes del repositorio principal), colapsando el nombre de workspace generado por Conductor, cualquier subpath y cualquier submódulo. Si no se puede resolver, el `project_name` MUST ser el segmento inmediatamente posterior a `workspaces` (`<repo>`).
5. En otro caso, el sistema SHALL usar la derivación previa (últimos uno o dos segmentos significativos).

#### Scenario: Worktree efímero de agente agrupado
- **WHEN** el `cwd` es `/Users/x/dev/inventures/tub2/.claude/worktrees/agent-abcd`
- **THEN** el `project_name` es `unknown`

#### Scenario: Worktree de agente dentro de un workspace de Conductor
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/tub2/kelowna-v1/FRONT/.claude/worktrees/agent-ad31`, aunque el repo `tub2` sea resoluble
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
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/tub2/chengdu-v4` y el índice de Conductor mapea ese workspace a `/Users/x/dev/inventures/tub2`
- **THEN** el `project_name` es `inventures/tub2`

#### Scenario: Ciudad con sufijo de versión
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/argus/cairo` o `.../argus/belo-horizonte-v1` y el repo `argus` resuelve a `/Users/x/dev/freelance/argus`
- **THEN** el `project_name` es `freelance/argus`

#### Scenario: Submódulo dentro del workspace
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/tub2/dili/FRONT/e2e/operator-assignment` y el índice resuelve `tub2`
- **THEN** el `project_name` es `inventures/tub2`

#### Scenario: Subpath profundo de monorepo
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/argus/cairo/packages/app/src-tauri`
- **THEN** el `project_name` es `freelance/argus`

#### Scenario: Workspace de Conductor sin resolver
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/clarus/oslo` y ninguna fuente resuelve el repo
- **THEN** el `project_name` es `clarus`

#### Scenario: Ruta ajena a Conductor
- **WHEN** el `cwd` no contiene `conductor/workspaces`, `.paseo/worktrees` ni un worktree de agente (ej. `/Users/x/dev/inventures/tub2`)
- **THEN** el sistema usa la derivación previa (últimos uno o dos segmentos)

#### Scenario: Sin cwd
- **WHEN** el registro no trae `cwd`
- **THEN** el `project_name` es `unknown`

### Requirement: Backfill de nombres de proyecto existentes
El sistema SHALL recomputar `project_name` para las filas ya almacenadas en `usage_events` a partir de su `project_path` (el `cwd` crudo persistido), mediante migraciones gateadas por `schema_version`. Cada migración MUST ser idempotente y no MUST requerir re-ingerir los logs originales. Cuando la regla de derivación cambie, el sistema SHALL introducir una nueva migración de backfill para que las filas históricas adopten la regla vigente. El backfill vigente MUST usar los índices de Paseo y Conductor cargados una vez al inicio de la migración y SHALL escribir sólo las filas cuyo `project_name` cambia. Un fallo al cargar cualquiera de los índices MUST NOT hacer fallar la migración. En la misma transacción, el backfill vigente SHALL remapear `project_group_members` para cada nombre antiguo cuyas filas pasaron todas a un único nombre nuevo, salvo `unknown`.

#### Scenario: Worktrees históricos corregidos
- **WHEN** existen filas cuyo `project_path` es un worktree de agente y cuyo `project_name` fue derivado por una regla antigua (ej. `worktrees/agent-XXXX`) y se aplica la migración vigente
- **THEN** esas filas quedan con `project_name = unknown`, sin alterar su `project_path`

#### Scenario: Filas de Paseo re-atribuidas
- **WHEN** existen filas con `project_name = unknown` cuyo `project_path` es un worktree de Paseo resoluble (ej. `/Users/x/.paseo/worktrees/3qkyl933/resolute-seal` → `mainRepoRoot = /Users/x/dev/freelance/tokenwatch`) y se aplica la migración vigente (v8)
- **THEN** esas filas quedan con `project_name = freelance/tokenwatch`, sin alterar su `project_path`

#### Scenario: Filas de Conductor re-atribuidas al repo principal
- **WHEN** existen filas con `project_name = tub2` y `project_path = /Users/x/conductor/workspaces/tub2/chengdu-v4`, el índice de Conductor resuelve ese workspace a `/Users/x/dev/inventures/tub2`, y se aplica la migración vigente (v8)
- **THEN** esas filas quedan con `project_name = inventures/tub2`, sin alterar su `project_path`

#### Scenario: Filas de Conductor no resolubles conservan el repo
- **WHEN** existen filas de un workspace de Conductor cuyo repo no resuelve por ninguna fuente
- **THEN** esas filas conservan `project_name = <repo>`

#### Scenario: Membresía de grupo remapeada
- **WHEN** `project_group_members` contiene `tub2` en el grupo G y todas las filas `tub2` pasan a `inventures/tub2`
- **THEN** `inventures/tub2` queda en G (si no tenía membresía) y la membresía `tub2` se elimina

#### Scenario: Membresía existente del nombre nuevo se respeta
- **WHEN** `backend` está en el grupo G1, `meki/backend` ya está en G2, y todas las filas `backend` pasan a `meki/backend`
- **THEN** `meki/backend` sigue en G2 y la membresía `backend` se elimina

#### Scenario: unknown no se remapea
- **WHEN** filas `unknown` pasan a varios repos distintos (Paseo)
- **THEN** la membresía de `unknown`, si existe, no cambia

#### Scenario: Migración idempotente
- **WHEN** una migración de backfill se ejecuta sobre una base cuyo `schema_version` ya la incluye
- **THEN** no se realizan cambios adicionales

#### Scenario: Versión de esquema tras la migración
- **WHEN** se abre una base nueva o existente con la versión vigente del binario
- **THEN** `schema_version` queda en `8`

### Requirement: Reconciliación del costo persistido ante cambios en la tabla de precios

El costo se persiste por evento al momento de la ingesta y no se recalcula al leer. Cuando la tabla de precios embebida o sus reglas de emparejamiento cambian (p. ej. el arreglo que hace que `claude-sonnet-5` resuelva a la familia `sonnet`), el sistema SHALL reconciliar el costo persistido de los eventos ya almacenados, recomputándolo a partir del `model` y los cuatro conteos de token guardados en cada fila, de modo que el arreglo repare el historial y no solo las ingestas nuevas.

La reconciliación SHALL ejecutarse automáticamente (mediante una migración de esquema versionada), MUST operar únicamente sobre datos ya persistidos en SQLite (sin reparsear el JSONL ni alterar los offsets de ingesta incremental), y MUST ser idempotente: correrla sobre datos ya correctos no cambia ningún valor. La reconciliación MUST modificar solo la columna `cost`, dejando intactos los conteos de token, `dedup_key`, `project_name` y el resto de columnas.

#### Scenario: Evento con costo cero por modelo antes desconocido se repara
- **WHEN** existe un evento almacenado con `model = "claude-sonnet-5"` y `cost = 0` (ingerido antes de que la familia resolviera), y se ejecuta la reconciliación
- **THEN** su `cost` pasa a `input·Pin + output·Pout + cache_creation·Pwrite + cache_read·Pread` con la fila de precios de la familia `sonnet`, quedando mayor que cero

#### Scenario: Los conteos de token y demás columnas no cambian
- **WHEN** la reconciliación recomputa el costo de un evento
- **THEN** los cuatro conteos de token, `total_tokens`, `dedup_key`, `project_name` y `timestamp` de esa fila permanecen sin cambios

#### Scenario: Reconciliación idempotente
- **WHEN** la reconciliación se ejecuta por segunda vez sobre eventos cuyo costo ya es el correcto según la tabla actual
- **THEN** ningún valor de `cost` cambia

#### Scenario: Modelo de familia desconocida permanece en cero
- **WHEN** la reconciliación procesa un evento cuyo `model` no pertenece a ninguna familia de la tabla (p. ej. `gpt-4o`)
- **THEN** su `cost` permanece en 0

#### Scenario: No se reparsea el JSONL
- **WHEN** se ejecuta la reconciliación
- **THEN** no se leen los archivos `~/.claude/projects/**/*.jsonl` ni se modifican los offsets de `ingest_files`, y la reparación funciona aunque el JSONL original ya no exista en disco

### Requirement: Resolución de proyecto para worktrees de Paseo
Cuando el `cwd` contiene los componentes consecutivos `.paseo` → `worktrees` seguidos de `<id>` y `<slug>` (layout `~/.paseo/worktrees/<id>/<slug>[/<subpath>]`), el sistema SHALL atribuir el evento al **repositorio principal** del worktree, aplicando esta cadena en orden:
1. Índice construido a partir de `~/.paseo/projects/workspaces.json` (array de entradas con `worktreeRoot`, `cwd` y `mainRepoRoot`, incluyendo worktrees archivados). El sistema SHALL buscar la entrada por el prefijo de ruta más largo (en límites de componente) del `cwd`, de modo que los subdirectorios del worktree también resuelvan. El `project_name` MUST derivarse de `mainRepoRoot` aplicando las reglas no-Paseo de derivación (p. ej. `/Users/x/dev/freelance/tokenwatch` → `freelance/tokenwatch`).
2. Si no hay entrada en el índice: el archivo `.git` del worktree root (`gitdir: <repo>/.git/worktrees/<x>`); el repo principal es la ruta anterior a `/.git/`, y el nombre se deriva igual que en el paso 1.
3. Si ninguno resuelve: el `project_name` MUST ser el segmento `<slug>`, nunca `unknown`.

El índice MUST cargarse una vez por corrida de ingesta (y una vez por backfill) y pasarse a la derivación, que MUST permanecer libre de E/S. Un `workspaces.json` ausente, ilegible, malformado o con formato inesperado MUST tratarse como índice vacío (con log) y MUST NOT provocar panic ni abortar la ingesta. El sistema MUST NOT usar el segmento `<id>` para derivar el nombre. El índice de Paseo también aporta roots principales conocidos para la resolución de Conductor (paso 4).

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

### Requirement: Resolución de proyecto para workspaces de Conductor
Cuando el `cwd` contiene los componentes consecutivos `conductor` → `workspaces` seguidos de `<repo>` y opcionalmente `<workspace>` (layout `.../conductor/workspaces/<repo>/<workspace>[/<subpath>]`), y no es un worktree efímero de agente, el sistema SHALL atribuir el evento al **repositorio principal**, aplicando esta cadena en orden:
1. La base interna de Conductor (`~/Library/Application Support/com.conductor.app/conductor.db`, tablas `workspaces.workspace_path` → `repos.root_path`): coincidencia exacta del workspace root (`.../conductor/workspaces/<repo>/<workspace>`).
2. El mapa `<repo>` → `repos.root_path` de la misma base (derivado de `repos.name` y del segmento `<repo>` de cada `workspace_path`), sólo si apunta a un único root.
3. El archivo `.git` del workspace root (`gitdir: <repo>/.git/worktrees/<x>`).
4. Un root principal conocido (unión de `mainRepoRoot` de Paseo y `root_path` de Conductor) cuyo nombre base sea exactamente `<repo>` y sea único.
5. Si ninguno resuelve: el segmento `<repo>`.
El `project_name` MUST derivarse del root principal con las reglas no-Paseo (p. ej. `/Users/x/dev/meki/backend` → `meki/backend`), de modo que sesiones directas, de Paseo y de Conductor del mismo repo compartan nombre. La base de Conductor MUST abrirse en sólo lectura (nunca escritura ni creación de archivos auxiliares) y cargarse una vez por corrida de ingesta y por backfill. Una base ausente, bloqueada, ilegible o con esquema inesperado MUST tratarse como índice vacío (con log) y MUST NOT provocar panic ni abortar la ingesta o la migración. Un `<repo>` o nombre base ambiguo (varios roots distintos) MUST NOT resolverse por los pasos 2 ni 4.

#### Scenario: Workspace presente en la base de Conductor
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/backend/brasilia` y la base de Conductor mapea ese `workspace_path` al repo con `root_path = /Users/x/dev/meki/backend`
- **THEN** el `project_name` es `meki/backend`

#### Scenario: Workspace archivado o renombrado resuelve por repo
- **WHEN** el `cwd` es `/Users/x/conductor/workspaces/backend/lima-v9`, ausente de `workspaces`, y `repos` contiene `backend` → `/Users/x/dev/meki/backend`
- **THEN** el `project_name` es `meki/backend`

#### Scenario: Fallback por archivo .git en Conductor
- **WHEN** el repo no está en la base de Conductor y el archivo `.git` del workspace contiene `gitdir: /Users/x/dev/freelance/argus/.git/worktrees/cairo`
- **THEN** el `project_name` es `freelance/argus`

#### Scenario: Repo eliminado de Conductor resuelto por root conocido
- **WHEN** el `cwd` es `.../conductor/workspaces/tokenwatch/oslo`, Conductor ya no conoce el repo, y el único root principal conocido con nombre base `tokenwatch` es `/Users/x/dev/freelance/tokenwatch`
- **THEN** el `project_name` es `freelance/tokenwatch`

#### Scenario: Nombre base ambiguo no se resuelve
- **WHEN** existen dos roots principales conocidos distintos con nombre base `backend` y el `<repo>` sólo podría resolverse por nombre base
- **THEN** el `project_name` es `backend`

#### Scenario: Base de Conductor ausente o ilegible
- **WHEN** `conductor.db` no existe, está corrupta o no tiene las tablas esperadas
- **THEN** el sistema usa un índice vacío, registra un log, no hace panic y continúa con los fallbacks

#### Scenario: Base de Conductor en WAL sin archivos auxiliares
- **WHEN** `conductor.db` está en modo WAL y no existen `-wal` ni `-shm` (Conductor cerrado)
- **THEN** el sistema igual lee la base en sólo lectura sin crear archivos auxiliares
