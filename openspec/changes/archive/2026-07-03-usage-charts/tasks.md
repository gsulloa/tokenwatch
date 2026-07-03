## 1. Precios (pricing.rs)

- [x] 1.1 Añadir deps al `Cargo.toml`: `rusqlite` (feature `bundled`), `glob`, `chrono`
- [x] 1.2 Crear `src/pricing.rs` con la tabla de precios embebida por modelo (input/output/cache_write/cache_read)
- [x] 1.3 Implementar `fn cost(model, usage) -> f64`; modelo desconocido → 0 + log
- [x] 1.4 Tests de `cost` con fixtures conocidas y test que documenta la fuente de precios

## 2. Base de datos (db/)

- [x] 2.1 Crear `src/db/mod.rs`: apertura de `tokenwatch.db` en el app-data dir (crate `directories`)
- [x] 2.2 Migraciones con `schema_version`: tablas `usage_events`, `ingest_files`, `meta` + índices
- [x] 2.3 `INSERT ... ON CONFLICT(dedup_key) DO NOTHING` (ingesta idempotente)
- [x] 2.4 Tests: creación de esquema, insert idempotente, lectura básica

## 3. Ingesta (ingest/)

- [x] 3.1 Crear `src/ingest/mod.rs`: walk de `~/.claude/projects/**/*.jsonl` con `glob`
- [x] 3.2 Struct serde laxa (`#[serde(default)]`) que lea solo lo necesario y filtre líneas sin `message.usage`
- [x] 3.3 Dedup por `"{message.id}:{requestId}"`; derivación de `project_name` desde `cwd` (+ caso `unknown`)
- [x] 3.4 Ingesta incremental: skip por `(size, mtime)`, parseo desde `lines_ingested`, actualizar offset
- [x] 3.5 Tests: fixture JSONL → filas esperadas, dedup, project_name/`unknown`, reparse incremental sin duplicar

## 4. Comandos y polling (usage/)

- [x] 4.1 Crear `src/usage/mod.rs`: tipos `SeriesQuery` / `SeriesResponse` / `Bucket` / `Metric` / `SeriesBy`
- [x] 4.2 `query_series` con agregación SQL (`strftime` día/semana/mes, `SUM`, `GROUP BY`) + relleno de buckets vacíos
- [x] 4.3 Comandos `refresh_usage` y `usage_meta`; estado `Mutex<Connection>` en `.setup()` de `lib.rs`
- [x] 4.4 Tarea `tokio` de polling (~30s) → refresh → emite evento `usage-updated`; ingesta full inicial
- [x] 4.5 Tests: bucketing SQL day/week/month, relleno de huecos, series por modelo/proyecto/modelo-proyecto

## 5. Frontend (features/usage)

- [x] 5.1 Añadir dep `recharts`; crear `src/features/usage/`
- [x] 5.2 `useUsageSeries.ts`: `invoke('query_series')`, escucha `usage-updated`, estados loading/error/empty
- [x] 5.3 `ChartControls.tsx`: 3 controles segmentados (bucket, métrica, series)
- [x] 5.4 `UsageChart.tsx`: `ResponsiveContainer` + `LineChart` con una `Line` por serie
- [x] 5.5 Integrar en `App.tsx` reemplazando el placeholder; mostrar `last_refresh_at`
- [x] 5.6 `format.ts`: formateo de tokens (K/M) y costo (USD)

## 6. Pulido

- [x] 6.1 Paleta estable por nombre de serie (hash → color)
- [x] 6.2 Top-N series + "otros" agrupado, indicando cuántas se agruparon (no truncar en silencio)
- [x] 6.3 Nota visible de "costo estimado" cuando la métrica es costo
- [x] 6.4 Verificar `pnpm typecheck && pnpm lint && pnpm test:run` y `cargo fmt/clippy/test`

## 7. Seguimiento (tareas finales)

- [x] 7.1 Abrir issue de GitHub para "soporte de Codex en gráficos" (fuera de v1), enlazando a esta change
