# Tasks — project-budgets

## 1. Esquema — migración v5

- [x] 1.1 En `src-tauri/src/db/mod.rs`, añadir migración v5: crear `project_groups (id INTEGER PK AUTOINCREMENT, name TEXT NOT NULL UNIQUE, budget_basis TEXT, budget_value REAL, created_at TEXT NOT NULL, CHECK(budget_basis IN ('share','usd') OR budget_basis IS NULL), CHECK(budget_basis IS NULL OR budget_value > 0))` y `project_group_members (project_name TEXT PRIMARY KEY, group_id INTEGER NOT NULL REFERENCES project_groups(id) ON DELETE CASCADE)`. Bump `CURRENT_SCHEMA_VERSION` a 5. Aditivo: no tocar `usage_events`. (Nota: `PRAGMA foreign_keys=ON` ya está en `db/mod.rs:35`; no requiere trabajo extra.)

## 2. Backend — cachear el LimitsSnapshot (prerrequisito, D7)

- [x] 2.1 Añadir `last_limits: std::sync::Mutex<Option<LimitsSnapshot>>` (o `tokio::sync::Mutex`) a `AppState`.
- [x] 2.2 En el poll de límites (`run_limits_and_emit` / `spawn_limits_polling_task` en `limits/mod.rs` + `lib.rs`), tras obtener el snapshot, escribirlo en `state.last_limits` antes de emitir `limits-updated`.
- [x] 2.3 Exponer un helper interno `current_session_window(state) -> Option<(String /*resets_at*/, /*window_start*/)>` que lea del cache; sin sesión → `None`.

## 3. Backend — CRUD de grupos (budgets)

- [x] 3.1 Crear módulo `src-tauri/src/budgets/mod.rs`. Structs serde: `Group { id: i64, name: String, budget_basis: Option<String>, budget_value: Option<f64> }`, `GroupWithMembers { group: Group, members: Vec<String> }`.
- [x] 3.2 `list_groups() -> Result<Vec<GroupWithMembers>, String>`: join `project_groups` ⨝ `project_group_members`.
- [x] 3.3 `create_group(name, budget_basis: Option<String>, budget_value: Option<f64>) -> Result<Group, String>`: validar nombre no vacío y único (mapear UNIQUE a error legible); si `budget_basis=Some`, validar `basis ∈ {share,usd}` y rango (`share`: 0<v≤100; `usd`: v>0). Insertar con `created_at` ISO.
- [x] 3.4 `update_group(id, name, budget_basis, budget_value) -> Result<(), String>`: mismas validaciones; error si el nombre colisiona con otro.
- [x] 3.5 `delete_group(id) -> Result<(), String>`: borrar el grupo (miembros caen por `ON DELETE CASCADE`).
- [x] 3.6 `assign_project(project_name, group_id) -> Result<(), String>`: upsert en `project_group_members`.
- [x] 3.7 `unassign_project(project_name) -> Result<(), String>`: borrar la fila (vuelve a "otros").
- [x] 3.8 Registrar los seis comandos en el `invoke_handler` de `lib.rs`.

## 4. Backend — cálculo de participación local por grupo (D8)

- [x] 4.1 Structs serde: `GroupBudgetRow { group_id: Option<i64>, name: String, budget_basis: Option<String>, budget_value: Option<f64>, window_cost_usd: f64, local_cost_share_pct: f64, measured_value: Option<f64> }` (`group_id: None` para "otros") y `GroupBudgetsSnapshot { rows: Vec<GroupBudgetRow>, window_start: String, origin: String /* session|rolling */ }`.
- [x] 4.2 `compute_group_budgets(state, conn) -> GroupBudgetsSnapshot`: obtener ventana vía `current_session_window` (D2); `origin=session` con `resets_at`, si no `rolling` con `now−5h`. `SELECT project_name, SUM(cost) FROM usage_events WHERE timestamp >= window_start GROUP BY project_name`; mapear a grupo o "otros"; acumular `window_cost_usd`; `total=Σ`; `local_cost_share_pct = total>0 ? cost/total*100 : 0`; `measured_value` según `budget_basis` (`share`→pct, `usd`→cost, `NULL`→None). Ordenar por `window_cost_usd` desc, "otros" al final.
- [x] 4.3 Comando `query_group_budgets(state) -> Result<GroupBudgetsSnapshot, String>` que llama a 4.2. Registrar en `invoke_handler`. NO hace fetch de red (usa el cache de la fase 2).

## 5. Backend — alertas por grupo (D9)

- [x] 5.1 Estado por grupo en `meta`, clave `budget_alert:<group_id>` = JSON `{ last_resets_at: String, fired: bool }`. Helpers de lectura/escritura reutilizando el acceso a `meta`.
- [x] 5.2 `evaluate_group_alerts(budgets, session_resets_at: Option<String>, conn)`: sólo si `origin=session` (hay `resets_at`). Por cada fila con `group_id` y `budget_basis` no-`None`: si `resets_at != last_resets_at` → `fired=false`, actualizar; si `measured_value >= budget_value` y `!fired` → devolver notificación (texto con base: "% del costo local de la sesión" o "$X en la sesión") y `fired=true`. Ignorar "otros" y grupos sin tope. En modo `rolling` no evaluar.
- [x] 5.3 Integrar en el poll de límites: tras las alertas globales, `compute_group_budgets` + `evaluate_group_alerts`; emitir vía el plugin de notification sólo si `alerts_muted == false`.

## 6. Frontend — tipos y hooks

- [x] 6.1 `src/features/budgets/types.ts`: `Group`, `GroupWithMembers`, `GroupBudgetRow`, `GroupBudgetsSnapshot` (camelCase: `budgetBasis`, `budgetValue`, `windowCostUsd`, `localCostSharePct`, `measuredValue`, `windowStart`, `origin`).
- [x] 6.2 Hook `useGroupBudgets`: invoca `query_group_budgets` al montar, se suscribe a `limits-updated` y `usage-updated` para re-invocar; expone `snapshot/loading/error` y `refresh()`.
- [x] 6.3 Funciones de mutación para el CRUD que refrescan el estado tras cada cambio.

## 7. Frontend — UI del popover (D10, D11)

- [x] 7.1 `GroupBudgetsSection` en `src/features/budgets/`: título "Costo local por grupo" + subtítulo "ventana actual de 5h"; por grupo, `windowCostUsd` (líder) + `localCostSharePct` (secundario); si tiene tope, `CapMeter` con unidad correcta (% o $); fila "otros" al final; se omite si no hay grupos. NO etiquetar como % de sesión de Anthropic.
- [x] 7.2 Insertar `GroupBudgetsSection` en `Popover.tsx` debajo de `LimitsSection`, visualmente separada del gauge global.

## 8. Frontend — editor de grupos en el dashboard

- [x] 8.1 Panel/pestaña "Grupos": listar grupos con tope (base+valor) y miembros; crear/renombrar/borrar; fijar/quitar tope (selector de base `share`/`usd` + valor); asignar/desasignar proyectos desde los `project_name` conocidos.
- [x] 8.2 Enlazar a las mutaciones de 6.3 y refrescar `useGroupBudgets` tras cada cambio.

## 8b. Fixes de revisión (autoplan — aplicar antes de codear)

- [x] 8b.1 (D13, HIGH) Corregir el bump de versión: bloque `version < 4` escribe literal `"4"`; nuevo bloque `version < 5` escribe `"5"`; correr cada paso de migración en transacción.
- [x] 8b.2 (D12, CRÍTICO — opción A) Normalizar `window_start` a `…Z`-millis con `to_rfc3339_opts(SecondsFormat::Millis, true)` y mantener el compare de columna simple (`timestamp >= ?1`) para preservar `idx_ue_timestamp`. Parsear `resets_at` con `parse_from_rfc3339`; si falla → fallback rolling. (No se añade columna `timestamp_ms`.)
- [x] 8b.7 (D19, opción A) Mantener `project_name` como clave de membresía + doc-comment en `backfill_project_names` declarando que cualquier re-derivación futura DEBE remapear `project_group_members.project_name` (old→new).
- [x] 8b.3 (D14/D15) Crear `CapMeter` (barra `measured/budget`, readout con unidad, color por cercanía al tope) en vez de reusar `LimitGauge`. La participación se renderiza con la gramática de `TodayByProjectList` (líder `formatCost(windowCostUsd)`, secundario `NN% costo local`). Retitular "Costo local por grupo" + "ventana 5h". Implementar estados: loading, error, sin grupos, `total==0`, `origin` rolling/session, sobre-tope, nombres largos. Sin emoji empty-state; tokens y `formatCost` existentes.
- [x] 8b.4 (D16) Editor en un `.panel` (no modal); base con `SegmentedControl`; valor con validación cliente-side espejo del backend; cada proyecto muestra su grupo actual (chip) o "sin grupo"; confirm inline al borrar. Añadir comando `list_project_names` (o reutilizar el set de proyectos existente) y registrarlo. [Backend: `list_project_names` command implemented and registered ✓; frontend editor done]
- [x] 8b.5 (D17) `last_limits` = `std::sync::Mutex<Option<LimitsSnapshot>>`; clonar y soltar antes de `conn`/`.await`. `query_limits` escribe el cache en `Ok`. `delete_group` borra `budget_alert:<id>` en la misma transacción. Poll: `compute + evaluate + persist` bajo un solo `conn.lock()`. `compute_group_budgets` materializa todos los grupos definidos primero. Membresías a HashMap en un solo SELECT. Índice `idx_pgm_group_id`.
- [x] 8b.6 (D18) Enforce `share ≤ 100` en `create_group`/`update_group` (+ CHECK compuesto opcional).

## 9. Verificación

- [x] 9.1 Tests Rust: participación local (incluye `total==0`, "otros", grupo definido sin actividad aparece en 0), ventana `session` vs fallback `rolling`, ausencia de multiplicación por `session.utilization`, unicidad 1-proyecto-1-grupo, `ON DELETE CASCADE`, validación de base/rango (pin 0, 100, 100.01, ≤0), y la máquina de alerta por grupo (cruce `share`, cruce `usd`, no re-disparo, reset al cambiar `resets_at`, sin alertas en `rolling`, mute).
- [x] 9.1b Tests Rust de los fixes de revisión: **borde de ventana** (evento en `resets_at−5h ± 1s/1ms` incluido/excluido, con timestamps en formato `…000Z`); `resets_at` en forma `+00:00` parsea y ventana correctamente; `resets_at` inválido → rolling; **migración** de una DB en `schema_version=3` llega a 5 y crea las tablas v5 (guarda D13); orfan `budget_alert:<id>` se limpia tras `delete_group` y un grupo recreado arranca `fired=false`; grupo con modelo sin precio contribuye 0 y no dispara cap `usd`; `query_group_budgets` no hace I/O de red.
- [x] 9.2 Tests frontend de `GroupBudgetsSection` (con/sin tope, base % vs $, sin grupos) y del hook.
- [x] 9.3 `pnpm typecheck && pnpm lint && pnpm test:run` y `cargo fmt/clippy/test` en verde.
- [ ] 9.4 Prueba manual: crear grupos con tope `share` y `usd`, asignar proyectos, verificar el popover junto al gauge global, y que la notificación (con la base correcta en el texto) dispara una sola vez al cruzar el tope.
