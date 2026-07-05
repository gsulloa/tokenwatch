## Why

Hoy la app muestra el % global de la sesión de 5h (dato real de Claude) y, por separado, el consumo por proyecto del día (dato local de los JSONL). Pero no responde la pregunta operativa del usuario: **"¿qué grupo de proyectos se está comiendo mi sesión de 5h ahora mismo, y estoy por quedarme sin cupo?"**. Se quiere agrupar proyectos con nombres propios, ver cuánto pesa cada grupo en la sesión actual, y recibir un aviso cuando un grupo cruza un tope. No hay bloqueo: es un monitor.

**Decisión de diseño clave (post-revisión).** El % global de la sesión viene opaco desde `/api/oauth/usage` y **no es atribuible por proyecto de forma confiable**: la ponderación interna de Anthropic no coincide con el costo retail de `pricing.rs` (los cache-reads valen 0.1x pero consumen sesión distinto; el plan de suscripción no comparte denominador con el precio API; y el uso fuera de Claude Code mueve el gauge global sin aparecer en los JSONL). Multiplicar el % global por la participación de costo local produciría un número que *parece* preciso pero está sesgado justo contra los grupos con más cache y para el usuario más intenso. Por eso el sistema mide lo que **sí puede afirmar con certeza**: la participación de cada grupo en el **costo local (USD) de la ventana de la sesión**, mostrada **junto al** gauge global real (que ya existe). Los topes son sobre bases honestas: % del costo local de la ventana, o USD absolutos.

## What Changes

- **Grupos de proyecto configurables.** Tablas nuevas `project_groups` y `project_group_members`. Cada grupo tiene nombre único y un tope opcional expresado como **base + valor**: `budget_basis ∈ {share, usd}` con `budget_value` (`share` = % del costo local de la ventana, `0 < v ≤ 100`; `usd` = USD absolutos en la ventana, `v > 0`); ambos `NULL` = sin tope. Un proyecto pertenece a **exactamente un** grupo (o a ninguno). Los proyectos sin asignar caen en un bucket implícito **"otros"** (sin tope). Comandos Tauri de CRUD: `list_groups`, `create_group`, `update_group`, `delete_group`, `assign_project`, `unassign_project`.
- **Métrica primaria = participación de costo local en la ventana de sesión.** Para cada grupo (más "otros"): `window_cost_usd` (suma del `cost` de sus proyectos en la ventana) y `local_cost_share_pct = window_cost_usd / total_window_cost × 100`. La ventana es `[session.resets_at − 5h, now]` cuando hay límites; si no, `[now − 5h, now]` (fallback rolling, sin dependencia dura de la API). **No se multiplica por `session.utilization`.**
- **Comando `query_group_budgets`.** Devuelve, por grupo (más "otros"), su `local_cost_share_pct`, `window_cost_usd`, `budget_basis`, `budget_value`, el `measured_value` según su base, la ventana usada y su origen (`session` o `rolling`). Sólo lectura sobre `usage_events` + membresías + el snapshot de límites cacheado.
- **Prerrequisito: cachear el `LimitsSnapshot`.** Hoy `query_limits` refetchea de red en cada llamada; `query_group_budgets` correría en cada `usage-updated` (30s) → round-trips y riesgo de 429. Se añade `Mutex<Option<LimitsSnapshot>>` a `AppState`, escrito por el poll de límites y leído por el comando nuevo (para obtener `session.resets_at` sin red).
- **Alertas por grupo (simple, advisory, base honesta).** Al cruzar hacia arriba su `budget_value` (según su `budget_basis`), **una** notificación por grupo por ventana de sesión, con la base explícita en el texto ("Grupo «Cliente A»: 31 % del costo local de la sesión (tope 30 %)" o "…: $2.10 en la sesión (tope $2.00)"). El estado se reinicia cuando cambia el `resets_at` de la sesión. Reutiliza el toggle `alerts_muted`. Grupos sin tope y "otros" nunca alertan. **Nunca bloquea.**
- **UI del popover.** Sección nueva "Uso por grupo (sesión)" debajo del gauge global: por grupo, `local_cost_share_pct` y `window_cost_usd`, más su progreso hacia el tope cuando lo tenga (medidor `measured_value / budget_value`), y la fila "otros". Etiquetada como **uso local de Claude Code** para no confundirla con el % global de Anthropic.
- **UI de configuración en el dashboard.** Panel para crear/renombrar/borrar grupos, fijar el tope (base + valor) y asignar proyectos. El popover sólo muestra estado; la edición vive en el dashboard.

## Capabilities

### New Capabilities
- `project-budgets`: Agrupación configurable de proyectos; medición de la participación de costo local de cada grupo en la ventana de la sesión de 5h (métrica honesta, sin reverse-engineering del % opaco de Anthropic); comando de estado; alertas advisory al cruzar un tope de base honesta (% de costo local o USD absoluto); y la UI de estado (popover, junto al gauge global existente) y de configuración (dashboard).

### Modified Capabilities
<!-- Ninguna capability existente cambia su comportamiento observable. El cacheo del último LimitsSnapshot en AppState (escrito por el poll de límites existente, leído por query_group_budgets sin red) es un prerrequisito interno que vive dentro del alcance de project-budgets; no altera query_limits ni las alertas globales. -->

<!-- El requirement "Cacheo del último snapshot de límites" se especifica en specs/project-budgets/spec.md. -->


## Impact

- **Rust (`src-tauri/src`)**: nuevo módulo `budgets/` (queries de costo por ventana, participación local, comandos CRUD y `query_group_budgets`, evaluación de alertas por grupo); nueva migración de esquema (v5) con `project_groups` + `project_group_members`; cacheo de `LimitsSnapshot` en `AppState` (escrito en el poll de límites); integración en el poll para disparar las alertas por grupo; registro de comandos en `invoke_handler`.
- **DB**: dos tablas nuevas; una clave por grupo en `meta` para el estado de alerta (`budget_alert:<group_id>`).
- **React (`src/`)**: `features/budgets/` con tipos espejo, hook `useGroupBudgets` (invoca `query_group_budgets`, escucha `limits-updated` y `usage-updated`), `GroupBudgetsSection` en el popover (bajo `LimitsSection`), y un editor de grupos en el dashboard.
- **No incluye**: multiplicar el % global por costo local (rechazado por no ser atribuible); bloqueo/throttling (es advisory); presupuestos sobre la ventana **semanal** (extensión futura, misma métrica local); un "estimated session-%" por grupo (evaluado como opción C y descartado en v1 para no reintroducir falsa precisión — puede reconsiderarse tras un spike de instrumentación que valide que los deltas de `session.utilization` siguen a los deltas de costo local).
