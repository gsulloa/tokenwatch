## 1. Derivación consciente de Conductor

- [x] 1.1 Reescribir `derive_project_name` en `src/ingest/mod.rs`: si los componentes contienen `conductor` seguido de `workspaces`, devolver el componente siguiente (`<repo>`); si no, mantener el fallback actual (últimos 1–2 segmentos); `cwd` vacío → `unknown`
- [x] 1.2 Tests: workspace root (`tub2/chengdu-v4`→`tub2`), ciudad con sufijo (`argus/belo-horizonte-v1`→`argus`), submódulo (`tub2/dili/FRONT/e2e/...`→`tub2`), subpath profundo (`argus/cairo/packages/app/src-tauri`→`argus`), ruta no-Conductor (fallback), sin cwd (`unknown`)

## 2. Backfill de filas existentes

- [x] 2.1 Añadir migración en `src/db/mod.rs` (bump `schema_version`): recomputar `project_name` desde `project_path` para todas las filas de `usage_events`, usando `ingest::derive_project_name`, dentro de una transacción
- [x] 2.2 Test: sembrar filas con `project_name` "antiguo" + `project_path` de Conductor → tras migrar, `project_name` corregido y `project_path` intacto; re-ejecutar no cambia nada (idempotente)

## 3. Verificación

- [x] 3.1 `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- [x] 3.2 Lanzar la app y confirmar que las series por proyecto se agrupan por repo (ej. un solo `tub2`, un solo `argus`), no por workspace
