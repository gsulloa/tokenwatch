# project-budgets Specification

## Purpose
Permite al usuario agrupar proyectos de Claude Code bajo nombres propios y fijarles topes opcionales expresados como porcentaje del costo local de la ventana de sesión (`share`) o en USD absolutos (`usd`). El sistema calcula, por cada grupo y por el bucket implícito "otros", la participación exacta del costo local en la ventana de 5h y, cuando hay sesión activa, un estimado de sesión ponderado (`local_cost_share_pct × session.utilization / 100`) que actúa como proxy advisory del peso del grupo en la sesión global. Los topes son estrictamente advisory: el sistema emite notificaciones nativas cuando el `measured_value` de un grupo cruza su tope, pero nunca bloquea, throttlea ni pausa sesiones. Las cifras se presentan de forma honesta, distinguiendo siempre el costo local del porcentaje global de sesión de Anthropic.

## Requirements

### Requirement: Grupos de proyecto con tope de base honesta
El sistema SHALL permitir definir grupos de proyecto, cada uno con un **nombre único** y un **tope opcional** expresado como una base (`budget_basis`) y un valor (`budget_value`). La base MUST ser `share` (porcentaje del costo local de la ventana de sesión, con `0 < budget_value <= 100`) o `usd` (USD absolutos consumidos en la ventana, con `budget_value > 0`); ambos nulos MUST interpretarse como "sin tope". El sistema MUST persistir los grupos en la tabla `project_groups` y MUST rechazar la creación o renombrado con un nombre que colisione con otro existente. El sistema MUST NOT derivar el tope del porcentaje global de sesión de Claude.

#### Scenario: Crear grupo con tope porcentual de costo local
- **WHEN** el usuario crea "Cliente A" con base `share` y valor 30
- **THEN** el grupo se persiste con `budget_basis = 'share'` y `budget_value = 30`

#### Scenario: Crear grupo con tope en USD
- **WHEN** el usuario crea "Cliente B" con base `usd` y valor 2.0
- **THEN** el grupo se persiste con `budget_basis = 'usd'` y `budget_value = 2.0`

#### Scenario: Crear grupo sin tope
- **WHEN** el usuario crea "Interno" sin tope
- **THEN** el grupo se persiste con `budget_basis` y `budget_value` nulos

#### Scenario: Nombre duplicado
- **WHEN** el usuario crea o renombra un grupo con un nombre que ya existe
- **THEN** el sistema rechaza la operación con un error y no modifica los datos

#### Scenario: Valor fuera de rango
- **WHEN** el usuario fija base `share` con valor 0 o mayor que 100, o base `usd` con valor 0 o negativo
- **THEN** el sistema rechaza la operación con un error

### Requirement: Un proyecto pertenece a exactamente un grupo
El sistema SHALL garantizar que cada `project_name` esté asignado a lo sumo a un grupo, forzándolo mediante `project_name` como clave primaria en `project_group_members`. Reasignar MUST reemplazar la membresía anterior. Desasignar MUST eliminar la membresía. Borrar un grupo MUST desasignar en cascada a sus proyectos.

#### Scenario: Asignar proyecto a grupo
- **WHEN** el usuario asigna "tub2" al grupo "Cliente A"
- **THEN** "tub2" queda como miembro de "Cliente A"

#### Scenario: Reasignar proyecto ya asignado
- **WHEN** el usuario asigna "tub2" (miembro de "Cliente A") al grupo "Experimentos"
- **THEN** "tub2" deja de ser miembro de "Cliente A" y pasa a "Experimentos"

#### Scenario: Borrar grupo con miembros
- **WHEN** el usuario borra un grupo con proyectos asignados
- **THEN** el grupo se elimina y sus proyectos quedan sin grupo (bucket "otros")

### Requirement: Bucket implícito "otros" para proyectos sin grupo
El sistema SHALL representar a los proyectos con actividad en la ventana que no pertenezcan a ningún grupo como un bucket sintético **"otros"** sin tope. Este bucket MUST NOT persistirse como grupo, MUST reflejar dinámicamente los proyectos no asignados presentes en la ventana, y MUST NOT generar alertas.

#### Scenario: Proyecto sin asignar aparece en "otros"
- **WHEN** un proyecto con costo en la ventana no pertenece a ningún grupo
- **THEN** su costo se agrega al bucket "otros"

#### Scenario: "otros" no alerta
- **WHEN** el bucket "otros" acumula cualquier costo o participación
- **THEN** el sistema no emite alertas para "otros"

### Requirement: Participación de costo local y estimado de sesión ponderado por grupo
El sistema SHALL calcular, por grupo (más "otros"), el costo local de la ventana `window_cost_usd` (suma del campo `cost` de los `usage_events` de sus proyectos dentro de la ventana) y su participación exacta `local_cost_share_pct = window_cost_usd / total_window_cost × 100`. El sistema MUST anclar la ventana a `[session.resets_at − 5h, now]` cuando el snapshot de límites cacheado tenga sesión, y MUST caer a `[now − 5h, now]` (rolling) cuando no la haya, reportando el `origin` (`session` o `rolling`). Cuando el costo total de la ventana sea 0, todas las participaciones MUST ser 0.

Adicionalmente, cuando `origin = session` y el snapshot tenga `session.utilization`, el sistema SHALL calcular un **estimado de sesión ponderado** por grupo `session_weighted_pct = local_cost_share_pct × session.utilization / 100`, y MUST reportarlo como `null` en modo `rolling`. `local_cost_share_pct` MUST permanecer como cantidad exacta (sin multiplicar por `session.utilization`); la multiplicación vive **solo** en `session_weighted_pct` (estimado advisory, ver D20).

#### Scenario: Participación por costo local
- **WHEN** en la ventana el costo es Cliente A $5, Experimentos $12, otros $3 (total $20)
- **THEN** `local_cost_share_pct` es Cliente A 25 %, Experimentos 60 %, otros 15 %, y `window_cost_usd` es $5, $12, $3

#### Scenario: Estimado de sesión ponderado
- **WHEN** un grupo tiene `local_cost_share_pct = 87 %` y la sesión está al 34 %
- **THEN** `session_weighted_pct` del grupo es ≈ 29.6 % (87 × 0.34), y la suma de `session_weighted_pct` de todos los grupos ≈ `session.utilization`

#### Scenario: Sin estimado ponderado en rolling
- **WHEN** el `origin` de la ventana es `rolling`
- **THEN** `session_weighted_pct` es `null` para todos los grupos

#### Scenario: Ventana anclada a la sesión
- **WHEN** el snapshot cacheado tiene `session.resets_at`
- **THEN** la ventana usa `[session.resets_at − 5h, now]` y `origin = session`

#### Scenario: Fallback rolling sin límites
- **WHEN** no hay snapshot de límites con sesión disponible
- **THEN** la ventana usa `[now − 5h, now]` y `origin = rolling`

#### Scenario: Ventana sin consumo local
- **WHEN** el costo total de la ventana es 0
- **THEN** la participación de todos los grupos es 0

#### Scenario: No se reconstruye el porcentaje global
- **WHEN** se calcula la participación por grupo
- **THEN** el resultado no incorpora `session.utilization` en ninguna fila

### Requirement: Cacheo del último snapshot de límites
El sistema SHALL cachear en memoria el último `LimitsSnapshot` obtenido por el poll de límites y SHALL exponerlo internamente para que `query_group_budgets` obtenga `session.resets_at` sin realizar una consulta de red por cada llamada. El poll de límites MUST escribir el snapshot tras cada consulta; el comando de presupuestos MUST leer del cache y MUST NOT gatillar un fetch de red propio.

#### Scenario: El poll actualiza el cache
- **WHEN** el poll de límites obtiene un snapshot
- **THEN** el snapshot queda disponible en el cache en memoria

#### Scenario: El comando lee del cache
- **WHEN** `query_group_budgets` necesita `session.resets_at`
- **THEN** lo obtiene del cache sin consultar la red

### Requirement: Comando de estado de presupuestos por grupo
El sistema SHALL exponer un comando `query_group_budgets` que devuelva, por cada grupo definido más "otros", su `local_cost_share_pct`, `session_weighted_pct` (estimado o `null` en rolling), `window_cost_usd`, `budget_basis`, `budget_value` y `measured_value`, junto con `window_start` y `origin` de la ventana. El `measured_value` MUST ser: base `share` → `session_weighted_pct` (el estimado ponderado; `null` en rolling), base `usd` → `window_cost_usd`, sin tope → `null`. El comando MUST ser de sólo lectura sobre `usage_events`, las membresías y el snapshot cacheado.

#### Scenario: Estado con grupos definidos
- **WHEN** el frontend invoca `query_group_budgets` con grupos definidos
- **THEN** recibe una fila por grupo (y "otros") con costo, participación, tope y `measured_value`, más `window_start` y `origin`

#### Scenario: Recalcular ante nuevos datos
- **WHEN** llega el evento `limits-updated` o `usage-updated`
- **THEN** el frontend re-invoca `query_group_budgets` y refleja el nuevo estado

### Requirement: Alerta advisory al cruzar el tope de un grupo
El sistema SHALL emitir una notificación nativa de macOS cuando el `measured_value` de un grupo con tope definido cruce hacia arriba su `budget_value`, y el texto MUST indicar la base del tope (para `share`, **estimado de sesión ponderado**; para `usd`, USD absolutos). El sistema MUST emitir **como máximo una** notificación por grupo por ventana de sesión, MUST reiniciar ese estado cuando cambie el `resets_at` de la sesión, MUST respetar el toggle `alerts_muted`, y MUST evaluar alertas sólo cuando el `origin` de la ventana sea `session` (no en modo rolling). Los grupos sin tope y "otros" MUST NOT alertar. El sistema MUST NOT bloquear, throttlear ni pausar sesiones.

#### Scenario: Primer cruce de un tope porcentual (estimado ponderado)
- **WHEN** el `session_weighted_pct` de "Cliente A" (tope `share` 30) pasa de 28 a 31
- **THEN** el sistema emite una notificación que menciona "31 % de tu sesión (est., tope 30 %)" y marca el grupo como notificado

#### Scenario: Alta participación local que NO cruza el tope de sesión
- **WHEN** "Mine" tiene `local_cost_share_pct = 87 %` pero la sesión está al 34 %, con tope `share` 30
- **THEN** su `session_weighted_pct` es ≈ 29.6 % (< 30), y el sistema **no** emite alerta

#### Scenario: Primer cruce de un tope en USD
- **WHEN** el `window_cost_usd` de "Cliente B" (tope `usd` 2.0) pasa de $1.80 a $2.10
- **THEN** el sistema emite una notificación que menciona "$2.10 en la sesión (tope $2.00)"

#### Scenario: Sin re-disparo en la misma ventana
- **WHEN** un grupo ya notificado sigue por encima de su tope en consultas posteriores de la misma ventana
- **THEN** el sistema no emite nuevas notificaciones para ese grupo

#### Scenario: Reinicio al renovarse la sesión
- **WHEN** el `resets_at` de la sesión cambia respecto al último visto
- **THEN** el sistema reinicia el estado de alerta de todos los grupos a "no notificado"

#### Scenario: Sin alertas en modo rolling
- **WHEN** la ventana está en modo `rolling` (sin sesión real)
- **THEN** el sistema no evalúa ni emite alertas por grupo

#### Scenario: Grupo sin tope
- **WHEN** un grupo sin tope acumula cualquier costo
- **THEN** el sistema no emite alertas para ese grupo

#### Scenario: Alertas silenciadas
- **WHEN** `alerts_muted` está activo y un grupo cruza su tope
- **THEN** el sistema no emite la notificación

### Requirement: Visualización del uso por grupo en el popover
El popover SHALL mostrar una sección "Uso por grupo", separada visualmente del gauge global de límites, con subtítulo que declare el modo: en `origin = session` **"estimado sobre tu sesión de 5h"**, en `rolling` **"costo local · ventana móvil de 5h (sin sesión activa)"**. Cada fila MUST liderar con `window_cost_usd` (vía el formateo de costo existente). Como valor secundario atenuado, en `origin = session` MUST mostrar el estimado ponderado `~{session_weighted_pct}% sesión`; en `rolling` MUST mostrar `{local_cost_share_pct}% costo local`. Cuando un grupo tenga tope y haya sesión, la fila MUST incluir un medidor cuya fracción sea `measured_value / budget_value` y cuyo readout lleve la unidad honesta (para `share`, `% sesión` etiquetado como estimado; para `usd`, `$`), coloreado por cercanía al tope propio y visualmente distinto del gauge de límites de Anthropic. La sección MUST omitirse cuando no haya grupos definidos. El estimado de sesión MUST rotularse como tal (p.ej. "est.") y NUNCA presentarse como el porcentaje de sesión exacto de Anthropic.

#### Scenario: Grupos con y sin tope (con sesión)
- **WHEN** con sesión activa existen "Cliente A" (share 30), "Cliente B" (usd 2.0) e "Interno" (sin tope)
- **THEN** el popover muestra el costo de cada grupo con su `~% sesión` estimado, un medidor de tope con readout `% sesión (est.)` para Cliente A, uno con readout `$` para Cliente B, "Interno" sin medidor, más la fila "otros"

#### Scenario: Alta participación local, estimado de sesión bajo el tope
- **WHEN** "Mine" es 87 % del costo local pero la sesión está al 34 %, con tope `share` 30
- **THEN** el popover muestra el medidor a ~29.6 % / 30 % sesión (no sobre el tope) en lugar de 87 %

#### Scenario: Sin grupos definidos
- **WHEN** no hay ningún grupo definido
- **THEN** la sección de costo por grupo no se muestra

#### Scenario: Sin consumo en la ventana
- **WHEN** hay grupos definidos pero el costo total de la ventana es 0
- **THEN** la sección muestra un estado vacío ("Sin consumo en esta ventana de 5h") en lugar de filas en 0

#### Scenario: Origen de la ventana visible
- **WHEN** el `origin` de la ventana es `rolling`
- **THEN** el popover indica de forma atenuada que es una ventana local móvil de 5h (no la sesión activa)

#### Scenario: Etiqueta honesta
- **WHEN** se muestra la sección de costo por grupo
- **THEN** queda claro que las cifras son de uso local de Claude Code y no el porcentaje global de sesión de Anthropic

### Requirement: Fuente de proyectos asignables
El sistema SHALL exponer la lista de `project_name` conocidos (por ejemplo vía un comando de sólo lectura sobre `usage_events`) para que el editor del dashboard pueda ofrecer los proyectos asignables y distinguir los no asignados.

#### Scenario: Listar proyectos conocidos
- **WHEN** el editor del dashboard se abre
- **THEN** puede obtener los `project_name` conocidos y marcar cuáles no pertenecen a ningún grupo

### Requirement: Configuración de grupos en el dashboard
El dashboard SHALL ofrecer una interfaz para crear, renombrar y borrar grupos, fijar o quitar su tope (base + valor), y asignar o desasignar proyectos conocidos. La edición MUST realizarse desde el dashboard (no desde el popover), y los cambios MUST reflejarse en el estado calculado por `query_group_budgets`.

#### Scenario: Crear y poblar un grupo
- **WHEN** el usuario crea un grupo, le fija un tope y le asigna proyectos desde el dashboard
- **THEN** el grupo y sus miembros se persisten y aparecen en el estado y en el popover

#### Scenario: El popover no edita grupos
- **WHEN** el usuario abre el popover
- **THEN** ve el estado por grupo pero no controles para crear, renombrar o borrar grupos
