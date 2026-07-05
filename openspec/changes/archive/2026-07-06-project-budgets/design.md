## Context

La app ya tiene: (a) el snapshot de límites reales de Claude vía `/api/oauth/usage` (`limits/mod.rs`), con la ventana de **sesión de 5h** (`session.utilization` %, `session.resets_at` ISO) y un poll cada 5 min que emite `limits-updated`; (b) ingesta de JSONL → SQLite `usage_events` con `project_name` derivado, `cost` (USD, calculado en `pricing.rs`) y `timestamp` UTC; (c) la máquina de alertas de umbral por ventana persistida en `meta`; y (d) un popover de barra de menú más un dashboard.

Este cambio pasó por revisión CEO dual (Claude + Codex). Ambas voces, de forma independiente, rechazaron la idea original de **atribuir el % global de la sesión a los grupos** repartiéndolo por costo local (ver `autoplan-review.md`). Resumen del porqué:

```
   session.utilization = 42 %  (GLOBAL, opaco, ponderación interna de Anthropic)
          ✗ no divisible con confianza por:
            · cache-reads valen 0.1x en pricing.rs pero consumen sesión distinto
            · suscripción ≠ precio API retail (no comparten denominador)
            · uso fuera de Claude Code mueve el gauge sin estar en los JSONL
   ⇒ multiplicar 42 % × (costo_grupo / costo_total) = número sesgado con falsa precisión
```

**Reencuadre adoptado (opción B):** medir sólo lo verificable — la participación de cada grupo en el **costo local de la ventana de sesión** — y mostrarla junto al gauge global real. Los topes son sobre bases honestas.

## Goals / Non-Goals

**Goals:**
- Definir grupos de proyecto con nombre propio y un tope opcional de base honesta (% del costo local de la ventana, o USD absolutos).
- Un proyecto pertenece a exactamente un grupo; lo no asignado cae en "otros" (sin tope).
- Calcular, por grupo, `local_cost_share_pct` y `window_cost_usd` sobre la ventana de la sesión de 5h (anclada a `session.resets_at`, con fallback rolling).
- Avisar (notificación macOS) una vez cuando un grupo cruza su tope, con la base explícita en el texto.
- Mostrar el estado por grupo en el popover (junto al gauge global) y configurarlo en el dashboard.

**Non-Goals:**
- Reverse-engineering del % global de Anthropic: nada de `session.utilization × cost_share`.
- Bloquear, throttlear o pausar sesiones (advisory).
- Presupuestos sobre la ventana **semanal** (misma métrica local aplicaría; fuera de v1).
- Un "estimated session-%" por grupo, ni siquiera etiquetado (opción C, descartada en v1).
- Un proyecto en varios grupos (rompe la partición).
- Cambiar la ingesta, `query_series`, o las alertas globales existentes.

## Decisions

### D1: Métrica primaria = participación de costo local en la ventana (no multiplicar el % global)
`local_cost_share_pct_grupo = window_cost_usd_grupo / total_window_cost × 100`, con `cost` sumado sobre `usage_events` cuyo `timestamp ∈ [window_start, now]`. Es una afirmación 100 % verificable ("Cliente A es el 60 % de lo que estás gastando en Claude Code esta ventana"). El gauge global de la sesión (existente) responde por separado "¿cuánto llevas de la sesión?". No se fusionan en un número derivado.
- *Rechazado (premisa original)*: `session.utilization × cost_share` → falsa precisión, sesgo contra grupos cache-heavy y para el usuario intenso (ver Context).
- *Rechazado (opción C)*: mostrar el número derivado como "estimate" secundario → reintroduce el riesgo de que el usuario lo lea como dato de Anthropic; se puede reconsiderar tras un spike de validación.

### D2: Ventana anclada a la sesión real, con fallback rolling
`window_start = session.resets_at − 5h` cuando el `LimitsSnapshot` cacheado tiene sesión; si no hay límites (no logueado, expirado, red caída), `window_start = now − 5h` (rolling local). Así la métrica primaria **no depende de forma dura** de la API de límites; el `origin` (`session | rolling`) se devuelve para que la UI pueda matizar. `now` en UTC; los `timestamp` en SQLite son UTC.

### D3: Base del tope honesta y explícita — `share` o `usd`
`budget_basis ∈ {share, usd}` con `budget_value`:
- `share`: `measured_value = local_cost_share_pct`; válido `0 < v ≤ 100`. Preserva la intención original del usuario ("grupo ≤ 30 %") pero medido sobre costo local, no sobre la sesión de Anthropic.
- `usd`: `measured_value = window_cost_usd`; válido `v > 0`. Directamente accionable para facturación ("Cliente A superó $2 esta sesión"), como pidieron ambas voces.
- ambos `NULL` = sin tope.
- *Por qué dos bases*: la revisión marcó que descartar los topes USD absolutos fue prematuro; `usage_events.cost` ya existe y da semántica clara. Un enum de una columna cubre ambos sin sobrecargar.

### D4: Un proyecto → un grupo, forzado por el esquema
`project_group_members.project_name` es **PRIMARY KEY**. Reasignar = upsert (reemplaza `group_id`). Desasignar = borrar la fila (vuelve a "otros"). Borrar grupo = `ON DELETE CASCADE`. Nota de fragilidad: `project_name` es un string derivado que migraciones previas re-derivan; si la derivación cambia, las membresías podrían orfanarse en "otros". Se acepta para v1 (mismo identificador que usa toda la app) y se documenta.

### D5: "otros" es un bucket implícito, no una fila
No se persiste. En `query_group_budgets`, todo `project_name` con costo en la ventana sin membresía se agrega a "otros" (`budget_basis = NULL`). Nunca alerta.

### D6: Esquema — dos tablas nuevas (migración v5)
```sql
CREATE TABLE project_groups (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  name         TEXT NOT NULL UNIQUE,
  budget_basis TEXT,               -- NULL | 'share' | 'usd'
  budget_value REAL,               -- share: 0<v<=100 ; usd: v>0 ; NULL si sin tope
  created_at   TEXT NOT NULL,
  CHECK (budget_basis IN ('share','usd') OR budget_basis IS NULL),
  CHECK (budget_basis IS NULL OR budget_value > 0)
);
CREATE TABLE project_group_members (
  project_name TEXT PRIMARY KEY,
  group_id     INTEGER NOT NULL REFERENCES project_groups(id) ON DELETE CASCADE
);
```
`PRAGMA foreign_keys` ya está ON (`db/mod.rs:35`), así que `ON DELETE CASCADE` funciona sin trabajo extra. Migración aditiva, no toca `usage_events`. Bump `schema_version` 4 → 5.

### D7: Prerrequisito — cachear el `LimitsSnapshot` en `AppState`
Hoy `query_limits` hace un GET a la red en cada llamada y el poll no guarda el resultado. `query_group_budgets` necesita `session.resets_at` y correrá seguido (en cada `usage-updated`, ~30s). Añadir `last_limits: Mutex<Option<LimitsSnapshot>>` a `AppState`; el poll de límites lo escribe tras cada fetch; `query_group_budgets` lo lee (sin red). Si está vacío → fallback rolling (D2). Esto evita round-trips y el riesgo de 429 que el propio código ya cuida.

### D8: Cálculo en `query_group_budgets` (sólo lectura)
1. Leer `last_limits` cacheado. `origin = session` si hay `session.resets_at`, si no `rolling`.
2. `window_start` según D2.
3. `SELECT project_name, SUM(cost) AS cost FROM usage_events WHERE timestamp >= window_start GROUP BY project_name`. (Reutiliza `idx_ue_timestamp`.)
4. `total = Σ cost`. Mapear cada `project_name` a su grupo (o "otros"); acumular `window_cost_usd` por grupo.
5. `local_cost_share_pct = total > 0 ? window_cost_usd/total*100 : 0`.
6. `measured_value` por grupo según `budget_basis` (`share`→pct, `usd`→cost, `NULL`→null).
7. Devolver filas `{ group_id, name, budget_basis, budget_value, window_cost_usd, local_cost_share_pct, measured_value }` + `{ window_start, origin }`, ordenadas por `window_cost_usd` desc, "otros" al final.

### D9: Alerta por grupo — simple, una por ventana, base explícita
Estado por grupo en `meta`, clave `budget_alert:<group_id>` = `{ last_resets_at, fired }`. En el poll de límites, tras computar los budgets:
- `window_id` = `session.resets_at` si `origin=session`; si `rolling`, no se evalúan alertas (sin sesión real no hay "ventana" estable que resetear — evita spam en rolling).
- Si `window_id != last_resets_at` → `fired = false`.
- Si `measured_value >= budget_value` y `!fired` y `!alerts_muted` → emitir **una** notificación con la base en el texto; `fired = true`.
- Grupos sin tope y "otros" no se evalúan.

### D10: Nombres de datos que no mienten
Campos: `local_cost_share_pct`, `window_cost_usd`, `budget_basis`, `budget_value`, `measured_value`, `origin`. **Nada** se llama `session_pct`. La sección del popover se titula "Uso por grupo (sesión)" con subtítulo "uso local de Claude Code", separada visualmente del gauge global de Anthropic.

### D11: UI — estado en popover, edición en dashboard
- **Popover**: `GroupBudgetsSection` bajo `LimitsSection`. Por grupo: `local_cost_share_pct` + `window_cost_usd`; si tiene tope, medidor `measured_value/budget_value` (reutiliza `LimitGauge`) con la unidad correcta (% o $); fila "otros" al final. Se omite si no hay grupos.
- **Dashboard**: editor de grupos (crear/renombrar/borrar, fijar tope base+valor, asignar proyectos desde los `project_name` conocidos).

## Risks / Trade-offs

- **[Cobertura local parcial]** Si el usuario consume Claude fuera de Claude Code, el gauge global sube sin costo local correspondiente. Con el reencuadre esto **ya no falsea** las cifras por grupo (sólo medimos costo local, no repartimos el global). Opcional futuro: una nota "el gauge global subió sin uso local reciente" para contexto.
- **[Modelos sin precio → cost 0]** No contribuyen al costo de la ventana → subrepresentan al grupo que los use. Mitiga: `pricing.rs` cubre los modelos actuales y loguea los desconocidos; considerar una nota "hay uso sin precio" si aparece.
- **[Fallback rolling vs. sesión]** En modo rolling la ventana no coincide exactamente con la sesión de Anthropic; por eso las alertas sólo corren en modo `session` (D9) y la UI muestra el `origin`.
- **[project_name como clave]** Ver D4: acoplamiento a la derivación; documentado.
- **[Suma de topes >100 %]** Con topes `share` independientes (30 %+60 %+sin tope) la suma puede exceder 100 %; intencional (avisos por grupo, no reparto de torta). Documentar en la UI.

## Migration Plan

1. Migración v5: `project_groups` + `project_group_members` (aditivo).
2. `AppState.last_limits` + escritura en el poll de límites (D7).
3. Módulo `budgets/`: queries, cálculo (D8), comandos CRUD y `query_group_budgets`; registrar en `invoke_handler`.
4. `evaluate_group_alerts` integrado en el poll (D9).
5. Frontend: `features/budgets/` (tipos, `useGroupBudgets`), `GroupBudgetsSection` en el popover, editor en el dashboard.
6. Verificación: `pnpm typecheck && pnpm lint && pnpm test:run` y `cargo fmt/clippy/test`.
- **Rollback**: comandos y UI aditivos; borrar las tablas v5, el cache `last_limits` y la sección del popover restaura el estado actual sin afectar ingesta ni límites.

## Decisiones de revisión (autoplan — Design + Eng dual voices)

Ambas fases (Claude + Codex) confirmaron los siguientes puntos. Los mecánicos se aplican; dos quedan como decisiones de gusto para el gate final (marcadas ⚑).

### D12 — Comparación de timestamps: NO comparar strings crudos (CRÍTICO) — RESUELTO: opción (a)
`usage_events.timestamp` guarda el string del JSONL verbatim (`"…T…:…:….000Z"`), pero `Utc::to_rfc3339()` emite `+00:00`. La comparación léxica `timestamp >= window_start` es incorrecta en el borde exacto de la ventana (`+` ordena antes que `Z`, los millis desplazan). **Decisión (gate D2 → A):** normalizar `window_start` a `…Z`-millis con `to_rfc3339_opts(SecondsFormat::Millis, true)` y **mantener el compare de columna simple** para preservar el índice `idx_ue_timestamp` en el hot path. Parsear `resets_at` con `parse_from_rfc3339` (tolera `Z` y `+00:00`); si falla → fallback rolling. El test de borde obligatorio (`resets_at−5h ± 1s/1ms`, timestamps `…000Z`) protege contra drift de formato; si alguna vez se detectan timestamps no uniformes, se escala a la columna `timestamp_ms` (rechazada aquí por blast radius innecesario dado que el JSONL de Claude Code es consistente).

### D13 — Bug de migración v4→v5 heredado (auto-decidido, aplicar)
`db/mod.rs:76` escribe `CURRENT_SCHEMA_VERSION.to_string()` en el bloque `version < 4` en vez del literal `"4"`. Al subir a 5, una DB en v3 quedaría marcada `"5"` tras correr sólo v4, saltándose la DDL de v5. Fix: bloque `version < 4` escribe literal `"4"`; nuevo bloque `version < 5` escribe `"5"`; correr cada paso en transacción.

### D14 — Popover: NO reusar `LimitGauge` para la participación; nuevo `CapMeter` (auto-decidido)
`LimitGauge` es "% hacia un techo peligroso" y hardcodea `%` (`LimitGauge.tsx:115`) + reloj de reset → no puede renderizar `$` y hace que la participación se lea como % de sesión de Anthropic. Fix: la participación se muestra con la gramática de `TodayByProjectList` (nombre a la izq., `formatCost(windowCostUsd)` primario + `NN% del costo local` secundario/atenuado). El tope usa un `CapMeter` nuevo y visualmente distinto: barra cuya fracción es siempre `measured_value/budget_value`, con el readout llevando la unidad honesta (`"18% / 30% costo local"` o `"$2.10 / $2.00"`), coloreado por cercanía **al tope propio**, nunca por magnitud de la participación.

### D15 — Título honesto y estados renderizados (auto-decidido)
Retitular la sección **"Costo local por grupo"** con subtítulo **"ventana actual de 5h"** (quitar "sesión" para no colisionar con el gauge global). Estados a especificar (espejo de `LimitsSection`/`TodayByProjectList`): `loading && !snapshot` ("Cargando…"), `error`, sin grupos (omitir sección), grupos con `total==0` ("Sin consumo en esta ventana de 5h"), `origin=rolling` (línea atenuada "ventana local móvil de 5h") vs `session` ("ventana de sesión actual"), sobre-tope (`$2.10 / $2.00` en rojo, barra clamp 100%), nombres largos (elipsis con `title`, como `TodayByProjectList`). Todo `$` vía `formatCost`; números `tabular-nums`; headers 11px/700/uppercase/0.06em; sin empty-states con emoji. `origin` debe volver en `query_group_budgets`.

### D16 — Editor del dashboard: patrón existente, no vocabulario nuevo (auto-decidido)
La app no tiene `<form>`, modal, tab ni dialog. El editor va en un `.panel` (clase existente) en el dashboard, no un modal. Toggle de base `share`/`usd` con el `SegmentedControl` de `ChartControls`; inputs/selects nativos ya estilados en `global.css`; el valor lleva sufijo `%`/prefijo `$` y valida cliente-side espejo del backend. Cada fila de proyecto muestra su grupo actual (chip) o "sin grupo" → la reasignación nunca es silenciosa y los no asignados ("otros") son visibles y asignables. Borrar grupo = confirm inline ("Sus proyectos vuelven a otros"). **Nuevo comando requerido:** `list_project_names` (o reutilizar el set de proyectos de `query_series`/today) como fuente de proyectos asignables.

### D17 — Concurrencia y ciclo de vida del estado de alerta (auto-decidido)
- `last_limits` = `std::sync::Mutex<Option<LimitsSnapshot>>` (NO tokio); clonar bajo el lock y soltar antes de tocar `conn` o cualquier `.await`. Nunca sostener dos locks a la vez ni un lock cruzando un `.await`.
- `query_limits` también escribe `last_limits` en `LimitsStatus::Ok` (elimina el punto ciego de 5 min al arranque; el botón de refresh siembra budgets).
- `delete_group` borra `budget_alert:<id>` de `meta` en la misma transacción (evita claves huérfanas y la supresión de alerta por reuso de id). Mantener `AUTOINCREMENT` como defensa extra.
- El poll corre `compute_group_budgets` + `evaluate_group_alerts` + persistencia bajo **un solo** `conn.lock()` (como `evaluate_alerts` hoy) para evitar la carrera poll/CRUD.
- `compute_group_budgets` **materializa todos los grupos definidos primero** (init `window_cost_usd=0`) y luego pliega el resultado del query → los grupos sin actividad en la ventana igual aparecen (no parpadean).
- Membresías: leer `SELECT project_name, group_id FROM project_group_members` a un HashMap una vez por compute (no N+1). Añadir índice `idx_pgm_group_id`.

### D18 — Enforcement del rango `share ≤ 100` (auto-decidido)
El `CHECK (budget_value > 0)` de D6 no cubre `share ≤ 100`. Enforce en Rust `create_group`/`update_group` (fuente de verdad) y, opcional, `CHECK` compuesto: `CHECK (budget_basis IS NULL OR (budget_value > 0 AND (budget_basis <> 'share' OR budget_value <= 100)))`.

### D19 — Clave de membresía: `project_name` vs `project_key` estable — RESUELTO: opción (a)
`project_name` es un string derivado re-computado en v2/v3/v4 (`backfill_project_names`). Si la regla cambia otra vez, las membresías se orfanan silenciosamente en "otros". **Decisión (gate D1 → A):** mantener `project_name` como clave de membresía en v1 + **contrato de migración**: todo backfill futuro que re-derive nombres MUST remapear `project_group_members.project_name` con el mapeo old→new, y se añade un doc-comment en `backfill_project_names` declarando esta dependencia. Se descarta introducir un `project_key` estable ahora (opción b) por tocar ingest/schema para un riesgo que sólo se materializa en una migración futura, ya cubierta por el contrato.

### D20 — Reversión informada: el cap `share` mide % de sesión ponderado (estimado)

**Contexto:** tras dogfoodar la implementación inicial, el usuario detectó que la métrica `local_cost_share_pct` es confusa como base del cap. Un grupo puede ser el 87 % del costo local mientras la sesión real de 5h está sólo al 34 %, por lo que en términos de sesión ese grupo representa ~29.6 %, no 87 %. Usar el 87 % como referencia del cap (D1 / opción B inicial) lleva a alertas que el usuario percibe como falsas alarmas.

**Decisión:** definir, por grupo, el **estimado ponderado por sesión**:
```
session_weighted_pct = local_cost_share_pct × session.utilization / 100
```
- Disponible SÓLO cuando `origin = "session"` y el snapshot cacheado tiene `session.utilization`. En modo `rolling` (sin sesión activa), es `null`.
- Sumado sobre todos los grupos ≈ `session.utilization` (buena verificación de consistencia).
- Es un **estimado** (la ponderación interna de Anthropic ≠ ratio de costo retail — la advertencia del CEO review sobre cache, suscripción ≠ API, etc. sigue vigente). Se muestra con "est." en la UI y se usa sólo para alertas advisory, nunca bloqueantes.
- `local_cost_share_pct` permanece como cantidad pura, exacta y sin multiplicar — no cambia su definición.

**El cap `share` ahora mide `session_weighted_pct`** como `measured_value`. Así, un cap de 30 % significa "≈ 30 % de tu sesión de 5h (est.)". El cap `usd` no cambia (USD absolutos en la ventana).

En modo rolling no hay estimado ponderado y, por tanto, las alertas de cap `share` no se evalúan (igual que antes: D9 ya sólo corría alertas en `session`; con `measured_value = None` para `share` en rolling, esto queda garantizado por la estructura de datos).

**Supersede:** la postura "no multiplicar `session.utilization × cost_share`" (D1) se aplica ahora sólo a `local_cost_share_pct`. Para el cap y el display de sesión, la multiplicación se acepta conscientemente como estimado explícitamente etiquetado, tras validación empírica del usuario.

**Escenario de regresión:** local share 87 % × sesión 34 % → weighted ≈ 29.58 % → un cap de 30 % NO dispara la alerta. Este caso tiene test en `budgets/mod.rs` (`test_share_cap_87pct_local_34pct_session_under_30pct_cap`).

## Open Questions

- **¿Nota de cobertura local?** (gauge global subió sin costo local) — útil pero no bloqueante; fuera de v1.
- **¿Extender a la semanal?** Misma métrica local con `weekly.resets_at − 7d`. Fuera de v1.
- **¿Reconsiderar el "estimated session-%" (opción C)?** Sólo tras un spike que loguee deltas de `session.utilization` vs deltas de costo local durante ~1 semana y confirme que el ratio es estable. Hasta entonces, no.
