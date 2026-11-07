## MODIFIED Requirements

### Requirement: Consulta de utilización de límites de Claude
El sistema SHALL consultar `GET https://api.anthropic.com/api/oauth/usage` con los headers `Authorization: Bearer <token>` y `anthropic-beta: oauth-2025-04-20`, y exponer un comando `query_limits` que devuelva la utilización de la ventana de **sesión de 5h**, de la **semana** (total) y de los **límites semanales por modelo**.

El sistema SHALL derivar cada ventana de forma independiente. Primero la busca en el array normalizado `limits[]` (`kind: session | weekly_all | weekly_scoped`, `percent`, `resets_at`, `scope.model`). Si la ventana no aparece ahí, la toma de los campos top-level:
- `five_hour` para la sesión;
- `seven_day` para la semana;
- `seven_day_opus` / `seven_day_sonnet` para los semanales por modelo, solo cuando `limits[]` no contenga ninguna entrada `weekly_scoped`.

La presencia de `limits[]` MUST NOT impedir el respaldo top-level de una ventana ausente. Cada semanal por modelo MUST etiquetarse con el nombre de su modelo (`scope.model.display_name`).

El parseo MUST ser tolerante a campos ausentes, adicionales o `null`, incluidos `limits: null` y un `five_hour` con `utilization` o `resets_at` en `null`. Una entrada individual mal formada de `limits[]` MUST descartarse sin invalidar el resto de la respuesta. Una ventana de sesión cuyo `resets_at` ya haya pasado MUST tratarse como ausente.

#### Scenario: Respuesta con sesión y semana
- **WHEN** el endpoint responde con una ventana `session` y una `weekly_all`
- **THEN** `query_limits` devuelve para cada una su porcentaje de utilización y su `resets_at`

#### Scenario: Sesión ausente en limits[] pero presente en five_hour
- **WHEN** `limits[]` no está vacío, no incluye ninguna entrada `session` y la respuesta incluye `five_hour` con `utilization` y un `resets_at` futuro
- **THEN** `query_limits` devuelve la sesión a partir de `five_hour` y la semana a partir de `limits[]`

#### Scenario: Sin sesión activa
- **WHEN** `limits[]` no incluye `session` y `five_hour` está ausente o es `null`
- **THEN** `query_limits` devuelve la sesión como sin dato, la semana normalmente y estado `ok`

#### Scenario: Campos top-level nulos
- **WHEN** la respuesta incluye `five_hour` con `utilization` o `resets_at` en `null`
- **THEN** el parseo no falla y la sesión se marca como sin dato

#### Scenario: limits nulo
- **WHEN** la respuesta trae `limits: null`
- **THEN** el parseo no falla y las ventanas se derivan de los campos top-level

#### Scenario: Sesión vencida
- **WHEN** la ventana de sesión obtenida tiene un `resets_at` anterior al momento actual
- **THEN** `query_limits` devuelve la sesión como sin dato

#### Scenario: Límites semanales por modelo presentes
- **WHEN** la respuesta incluye una o más entradas `weekly_scoped` con `scope.model`
- **THEN** `query_limits` devuelve una ventana semanal por cada modelo, etiquetada con el nombre del modelo, con su porcentaje y `resets_at`

#### Scenario: Sin límites por modelo
- **WHEN** la respuesta no incluye ninguna entrada `weekly_scoped`
- **THEN** `query_limits` devuelve la lista de semanales por modelo vacía y las demás ventanas normalmente

#### Scenario: Ventana ausente en la respuesta
- **WHEN** una de las ventanas no aparece ni en `limits[]` ni en los campos top-level (p.ej. `session` es null)
- **THEN** esa ventana se marca como sin dato y las demás se devuelven normalmente

#### Scenario: Entrada mal formada en limits[]
- **WHEN** una entrada de `limits[]` tiene un tipo inesperado (p.ej. `percent` como texto)
- **THEN** esa entrada se ignora y las demás ventanas se devuelven normalmente

#### Scenario: Campos desconocidos
- **WHEN** la respuesta trae campos adicionales no modelados
- **THEN** el sistema los ignora y parsea los campos requeridos sin error

#### Scenario: Fallo de red o HTTP no exitoso
- **WHEN** la consulta falla por red o devuelve un status no exitoso
- **THEN** el sistema devuelve estado no disponible con el motivo correspondiente y no crashea el poll
