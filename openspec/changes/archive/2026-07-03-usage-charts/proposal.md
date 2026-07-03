## Why

TokenWatch todavía no muestra ningún dato de consumo: solo existe el scaffold. Los logs de uso de Claude viven en `~/.claude/projects/**/*.jsonl` y **no son permanentes** (se rotan/borran), por lo que necesitamos extraerlos a una base local para poder graficar consumo histórico de tokens y costo por proyecto, modelo y periodo.

## What Changes

- Parser nativo en Rust que lee los JSONL de `~/.claude/projects/**/*.jsonl` (registros `assistant`), sin depender de `ccusage` ni de `node` en runtime.
- Persistencia en **SQLite local** (`rusqlite` embebido) con grano por-mensaje, deduplicado por `message.id:requestId`, que **preserva la historia aunque el JSONL original se borre**.
- Cálculo de **costo estimado** con una tabla de precios embebida en Rust (input/output/cache-write/cache-read por modelo).
- Ingesta **incremental** (offset por archivo según size/mtime) + tarea de **polling** (~30s) que emite un evento Tauri `usage-updated`.
- Comando `query_series` que agrega en SQL el cruce **día/semana/mes × tokens/costo × modelo/proyecto/modelo-proyecto**, rellenando buckets vacíos.
- Frontend con gráfico de línea (`recharts`) + 3 controles segmentados (eje X, métrica, series), top-N series + "otros", y nota de "costo estimado".
- v1 **solo Claude** (Codex queda fuera; se abre issue de seguimiento como tarea final).

## Capabilities

### New Capabilities
- `usage-ingestion`: extracción, deduplicación y persistencia incremental de los eventos de uso desde los JSONL de Claude hacia SQLite, con cálculo de costo estimado y polling.
- `usage-charts`: consulta agregada de series temporales y visualización del gráfico de línea con controles de agrupación (bucket), métrica y series.

### Modified Capabilities
<!-- Ninguna: no existen specs previas. -->

## Impact

- **Backend (`packages/app/src-tauri`):** nuevos módulos `pricing.rs`, `db/`, `ingest/`, `usage/`; nuevas deps `rusqlite` (bundled), `glob`, `chrono`; estado `Mutex<Connection>` e inicialización de polling en `lib.rs`.
- **Frontend (`packages/app/src`):** nueva dep `recharts`; nuevos componentes en `src/features/usage/`; se reemplaza el placeholder en `App.tsx`.
- **Datos:** nuevo archivo `tokenwatch.db` en el app-data dir (crate `directories`).
- **Seguimiento:** issue de GitHub para "soporte de Codex en gráficos" (fuera de v1).
