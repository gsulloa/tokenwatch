## Context

TokenWatch es un scaffold Tauri 2 + React sin funcionalidad de monitoreo aún. Claude escribe logs de uso en `~/.claude/projects/<proj-dir>/<sessionId>.jsonl` (JSONL, append-only). Los registros `type: "assistant"` traen todo lo necesario — verificado empíricamente contra datos reales (515 sesiones):

```jsonc
{
  "type": "assistant", "requestId": "req_011C...", "sessionId": "d964...",
  "cwd": "/Users/x/conductor/workspaces/backend/madrid",      // proyecto real
  "timestamp": "2026-07-03T09:32:46.326Z",                     // por-mensaje
  "message": {
    "id": "msg_01Wv...", "model": "claude-opus-4-8",
    "usage": { "input_tokens": 10109, "output_tokens": 131,
      "cache_creation_input_tokens": 23200, "cache_read_input_tokens": 14334 }
  }
}
```

Estos logs **no son permanentes**, así que hay que extraerlos a una DB local antes de que se roten. Constraint del repo: `pnpm typecheck && pnpm lint && pnpm test:run` y `cargo fmt/clippy/test` deben pasar.

## Goals / Non-Goals

**Goals:**
- Gráfico de línea con eje X (día/semana/mes), métrica Y (tokens/costo) y series (modelo/proyecto/modelo-proyecto).
- Persistencia local en SQLite que sobreviva al borrado de los JSONL.
- Ingesta incremental + polling, todo nativo en Rust (sin `ccusage`, sin `node` en runtime).
- Costo estimado a partir de tabla de precios embebida.

**Non-Goals:**
- Soporte de Codex (v1 solo Claude → issue de seguimiento).
- Split de tarifas de caché ephemeral 1h vs 5m (se usa tarifa write única).
- File-system watch con `notify` (v1 usa polling simple).
- Alertas/límites de consumo (fuera de este v1).

## Decisions

**Fuente: JSONL nativo, no `ccusage`.** El JSONL crudo da `cwd` (nombre de proyecto limpio), `timestamp` por-mensaje (bucketing exacto, sin aproximar por sesión) y `message.id`+`requestId` (dedup idéntica a ccusage). Solo debemos calcular costo. *Alternativa descartada:* `ccusage session --json` — funcionaría pero añade dependencia de `node`/`npx` en runtime, problemática con el PATH reducido de apps GUI en macOS.

**Grano por-mensaje en SQLite.** Tabla `usage_events`, PK = `"{message_id}:{request_id}"`, ingesta `INSERT ... ON CONFLICT DO NOTHING` → idempotente y naturalmente deduplicada. *Alternativa descartada:* grano por sesión (como ccusage) — pierde precisión temporal en sesiones que cruzan medianoche.

**Costo: tabla de precios embebida (`pricing.rs`).** `cost = input·Pin + output·Pout + cache_creation·Pwrite + cache_read·Pread` (USD/token). Multiplicadores de caché estándar Anthropic (write 1.25×, read 0.1× del input). Modelo fuera de tabla → costo 0 + log (no rompe). Costo se marca como **estimado** en la UI.

**Ingesta incremental + polling.** Tabla `ingest_files(path, size, mtime, lines_ingested)`; si `(size, mtime)` no cambió se salta el archivo, si creció se parsea solo desde `lines_ingested` (append-only). Tarea `tokio` cada ~30s corre refresh y emite evento Tauri `usage-updated`; ingesta full al arrancar.

**Agregación en SQL.** `query_series` usa `strftime` sobre `timestamp` (Day `%Y-%m-%d`, Week `%Y-W%W`, Month `%Y-%m`), `SUM(cost)`/`SUM(total_tokens)` y `GROUP BY` por modelo / project_name / `model·project`. Backend rellena buckets vacíos con 0. Frontend aplica top-N series + "otros".

**Stack:** `rusqlite` (feature `bundled`, sin dep de sistema), `glob`, `chrono`; `directories` (ya presente) para el app-data dir. Frontend `recharts` (MIT).

## Risks / Trade-offs

- **Precios desactualizados o modelo nuevo** → costo 0 o incorrecto. *Mitigación:* test que documenta la fuente de precios; modelo desconocido loggea y no rompe; UI dice "estimado".
- **Tarifa de caché única (sin split 1h/5m)** → ligera sub/sobre-estimación. *Mitigación:* documentado como aproximación conocida; refinamiento futuro.
- **Cientos de proyectos/series saturan el gráfico** → top-N + "otros" agrupado, mostrando cuántas se agruparon (no truncar en silencio).
- **Polling cada 30s sobre muchos archivos** → costo I/O. *Mitigación:* skip por `(size, mtime)` y parseo incremental por offset.
- **Formato JSONL cambia entre versiones de Claude** → deserialización laxa (`#[serde(default)]`), se ignoran líneas sin `message.usage`.

## Open Questions

- Intervalo de polling final (default ~30s) — ajustable tras dogfooding.
- Cuántas series mostrar antes de agrupar en "otros" (default top 8).
