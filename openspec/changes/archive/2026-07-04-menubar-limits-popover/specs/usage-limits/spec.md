## ADDED Requirements

### Requirement: Autenticación vía Keychain de macOS
El sistema SHALL obtener el token OAuth de Claude leyendo el generic-password del Keychain de macOS con `service = "Claude Code-credentials"`, extrayendo `claudeAiOauth.accessToken` del valor JSON. El sistema MUST leer el token en cada ciclo de consulta (no cachearlo indefinidamente) y MUST tratar la ausencia del ítem, un valor no parseable o un permiso denegado como un estado no disponible, sin interrumpir el resto de la app.

#### Scenario: Token presente y válido
- **WHEN** el Keychain contiene `Claude Code-credentials` con un `accessToken` cuyo `expiresAt` es futuro
- **THEN** el sistema usa ese token para autenticar la consulta de límites

#### Scenario: Ítem de Keychain ausente
- **WHEN** no existe el ítem `Claude Code-credentials`
- **THEN** el sistema devuelve estado no disponible con motivo `not_signed_in` y no lanza error

#### Scenario: Permiso de Keychain denegado
- **WHEN** el usuario deniega el acceso del app al ítem de Keychain
- **THEN** el sistema devuelve estado no disponible con motivo `keychain_denied`

#### Scenario: Token expirado
- **WHEN** el `expiresAt` del token es anterior al momento actual
- **THEN** el sistema devuelve estado no disponible con motivo `expired` y NO intenta refrescar el token

### Requirement: Consulta de utilización de límites de Claude
El sistema SHALL consultar `GET https://api.anthropic.com/api/oauth/usage` con los headers `Authorization: Bearer <token>` y `anthropic-beta: oauth-2025-04-20`, y exponer un comando `query_limits` que devuelva la utilización de la ventana de **sesión de 5h**, de la **semana** (total) y de los **límites semanales por modelo**. El sistema SHALL derivar estas ventanas del array normalizado `limits[]` (`kind: session | weekly_all | weekly_scoped`, `percent`, `resets_at`, `scope.model`), cayendo a los campos top-level `five_hour`/`seven_day` cuando `limits[]` esté ausente. Cada semanal por modelo MUST etiquetarse con el nombre de su modelo (`scope.model.display_name`). El parseo MUST ser tolerante a campos ausentes o adicionales.

#### Scenario: Respuesta con sesión y semana
- **WHEN** el endpoint responde con una ventana `session` y una `weekly_all`
- **THEN** `query_limits` devuelve para cada una su porcentaje de utilización y su `resets_at`

#### Scenario: Límites semanales por modelo presentes
- **WHEN** la respuesta incluye una o más entradas `weekly_scoped` con `scope.model`
- **THEN** `query_limits` devuelve una ventana semanal por cada modelo, etiquetada con el nombre del modelo, con su porcentaje y `resets_at`

#### Scenario: Sin límites por modelo
- **WHEN** la respuesta no incluye ninguna entrada `weekly_scoped`
- **THEN** `query_limits` devuelve la lista de semanales por modelo vacía y las demás ventanas normalmente

#### Scenario: Ventana ausente en la respuesta
- **WHEN** la respuesta no incluye una de las ventanas (p.ej. `session` es null)
- **THEN** esa ventana se marca como sin dato y las demás se devuelven normalmente

#### Scenario: Campos desconocidos
- **WHEN** la respuesta trae campos adicionales no modelados
- **THEN** el sistema los ignora y parsea los campos requeridos sin error

#### Scenario: Fallo de red o HTTP no exitoso
- **WHEN** la consulta falla por red o devuelve un status no exitoso
- **THEN** el sistema devuelve estado no disponible con el motivo correspondiente y no crashea el poll

### Requirement: Polling de límites cada 5 minutos
El sistema SHALL ejecutar en segundo plano una tarea que consulta los límites cada 5 minutos y, al obtener un snapshot, emite el evento Tauri `limits-updated` con la utilización de sesión y semana. Este poll MUST ser independiente del poll de ingesta de JSONL existente.

#### Scenario: Ciclo periódico
- **WHEN** transcurren 5 minutos desde la última consulta de límites
- **THEN** el sistema vuelve a consultar el endpoint y emite `limits-updated`

#### Scenario: Independencia del poll de ingesta
- **WHEN** el poll de ingesta de JSONL corre en su intervalo
- **THEN** el poll de límites mantiene su propio intervalo de 5 minutos sin acoplarse a aquel

### Requirement: Alertas de umbral en sesión, semana y semana por modelo
El sistema SHALL emitir una notificación nativa de macOS cuando la utilización de la sesión (5h), de la semana total y/o de cualquier semanal por modelo cruce hacia arriba los umbrales **50 %, 70 % y 80 %**. El seguimiento de umbrales MUST ser independiente por ventana, identificando cada semanal por modelo por su modelo. El sistema MUST emitir como máximo una notificación por umbral y por ventana (identificada por su `resets_at`), y MUST reiniciar el seguimiento de umbrales cuando el `resets_at` de una ventana cambia (nueva ventana). Las notificaciones MUST distinguir a qué ventana corresponden (incluido el nombre del modelo cuando aplique).

#### Scenario: Primer cruce de un umbral
- **WHEN** la utilización de la semana pasa de 48 % a 56 %
- **THEN** el sistema emite una notificación indicando "semana: 50 % usado" y registra 50 como umbral ya notificado para esa ventana

#### Scenario: Sin re-disparo dentro de la misma ventana
- **WHEN** la utilización ya notificada al 50 % oscila entre 51 % y 69 % en consultas posteriores de la misma ventana
- **THEN** el sistema no emite nuevas notificaciones hasta cruzar el siguiente umbral (70 %)

#### Scenario: Salto de varios umbrales de una vez
- **WHEN** la utilización pasa de 40 % a 82 % entre dos consultas
- **THEN** el sistema emite una sola notificación correspondiente al umbral más alto cruzado (80 %)

#### Scenario: Reinicio al renovarse la ventana
- **WHEN** el `resets_at` de una ventana cambia respecto al último visto
- **THEN** el sistema reinicia los umbrales notificados de esa ventana a cero

#### Scenario: Umbrales independientes por ventana
- **WHEN** la sesión cruza 50 % pero la semana sigue por debajo de 50 %
- **THEN** el sistema notifica sólo la sesión y mantiene el seguimiento de la semana intacto

#### Scenario: Cruce de umbral en un semanal por modelo
- **WHEN** el semanal del modelo "Opus" cruza 70 % mientras la semana total sigue por debajo de 70 %
- **THEN** el sistema emite una notificación que identifica el modelo (p.ej. "Semana Opus: 70 % usado") sin afectar el seguimiento de las demás ventanas

### Requirement: Silenciar alertas
El sistema SHALL ofrecer un ajuste persistente para silenciar todas las notificaciones de umbral. Con el ajuste activo, el sistema MUST seguir consultando y mostrando la utilización, pero NO MUST emitir notificaciones.

#### Scenario: Alertas silenciadas
- **WHEN** el usuario activa "silenciar alertas" y luego se cruza un umbral
- **THEN** el sistema no emite notificación, pero el popover sigue reflejando la utilización actualizada

#### Scenario: Persistencia del ajuste
- **WHEN** el usuario silencia las alertas y reinicia la app
- **THEN** el ajuste sigue activo tras el reinicio
