## 1. Backend — enum y query

- [x] 1.1 Añadir la variante `Group` al enum `SeriesBy` en `packages/app/src-tauri/src/usage/mod.rs` (queda como `group` por `serde(rename_all = "camelCase")`).
- [x] 1.2 En `query_series_inner`, construir `from_clause` condicional: para `SeriesBy::Group` usar `usage_events e LEFT JOIN project_group_members m ON m.project_name = e.project_name LEFT JOIN project_groups g ON g.id = m.group_id`; para las demás ramas mantener `usage_events` sin alias (SQL idéntico al actual).
- [x] 1.3 Definir `series_col` para `SeriesBy::Group` como `COALESCE(g.name, 'otros')`; verificar que solo `project_name` es ambigua y queda cualificada como `e.project_name` en el ON del JOIN.
- [x] 1.4 Sustituir el literal `FROM usage_events` del SQL principal por `from_clause`, sin tocar `all_buckets_in_range` (sigue operando sobre `usage_events` con el `where_clause` de `timestamp` desnudo).

## 2. Backend — tests

- [x] 2.1 Test `query_series` con `series_by=Group`: dos grupos con proyectos asignados devuelven una serie por grupo con la suma correcta por bucket.
- [x] 2.2 Test: eventos de proyectos sin grupo se colapsan en una única serie `"otros"`.
- [x] 2.3 Test: proyecto reasignado a otro grupo se atribuye al grupo vigente.
- [x] 2.4 Test: sin membresías definidas, todo el uso cae en una sola serie `"otros"`.
- [x] 2.5 Verificar que los tests existentes de `query_series_inner` (model/project/modelProject) siguen pasando sin cambios.

## 3. Frontend — tipo y selector

- [x] 3.1 Extender `SeriesBy` en `packages/app/src/features/usage/types.ts` con `"group"`.
- [x] 3.2 Añadir `{ value: "group", label: "Grupo" }` a `SERIES_BY_OPTIONS` en `ChartControls.tsx`.
- [x] 3.3 Actualizar/añadir caso en `ChartControls.test.tsx` que cubra la opción "Grupo".
- [x] 3.4 Verificar que gráfico, tabla y colores renderizan la serie "otros" y las series por grupo sin cambios adicionales (consumo por `name`).

## 4. Verificación

- [x] 4.1 `pnpm typecheck && pnpm lint && pnpm test:run` en `packages/app`.
- [x] 4.2 `cargo fmt`, `cargo clippy` y `cargo test` en `packages/app/src-tauri`.
- [ ] 4.3 Smoke manual: en el dashboard, seleccionar Series=Grupo y confirmar que las bandas/filas usan nombres de grupo y "otros".
