## ADDED Requirements

### Requirement: Reconciliación del costo persistido ante cambios en la tabla de precios

El costo se persiste por evento al momento de la ingesta y no se recalcula al leer. Cuando la tabla de precios embebida o sus reglas de emparejamiento cambian (p. ej. el arreglo que hace que `claude-sonnet-5` resuelva a la familia `sonnet`), el sistema SHALL reconciliar el costo persistido de los eventos ya almacenados, recomputándolo a partir del `model` y los cuatro conteos de token guardados en cada fila, de modo que el arreglo repare el historial y no solo las ingestas nuevas.

La reconciliación SHALL ejecutarse automáticamente (mediante una migración de esquema versionada), MUST operar únicamente sobre datos ya persistidos en SQLite (sin reparsear el JSONL ni alterar los offsets de ingesta incremental), y MUST ser idempotente: correrla sobre datos ya correctos no cambia ningún valor. La reconciliación MUST modificar solo la columna `cost`, dejando intactos los conteos de token, `dedup_key`, `project_name` y el resto de columnas.

#### Scenario: Evento con costo cero por modelo antes desconocido se repara

- **WHEN** existe un evento almacenado con `model = "claude-sonnet-5"` y `cost = 0` (ingerido antes de que la familia resolviera), y se ejecuta la reconciliación
- **THEN** su `cost` pasa a `input·Pin + output·Pout + cache_creation·Pwrite + cache_read·Pread` con la fila de precios de la familia `sonnet`, quedando mayor que cero

#### Scenario: Los conteos de token y demás columnas no cambian

- **WHEN** la reconciliación recomputa el costo de un evento
- **THEN** los cuatro conteos de token, `total_tokens`, `dedup_key`, `project_name` y `timestamp` de esa fila permanecen sin cambios

#### Scenario: Reconciliación idempotente

- **WHEN** la reconciliación se ejecuta por segunda vez sobre eventos cuyo costo ya es el correcto según la tabla actual
- **THEN** ningún valor de `cost` cambia

#### Scenario: Modelo de familia desconocida permanece en cero

- **WHEN** la reconciliación procesa un evento cuyo `model` no pertenece a ninguna familia de la tabla (p. ej. `gpt-4o`)
- **THEN** su `cost` permanece en 0

#### Scenario: No se reparsea el JSONL

- **WHEN** se ejecuta la reconciliación
- **THEN** no se leen los archivos `~/.claude/projects/**/*.jsonl` ni se modifican los offsets de `ingest_files`, y la reparación funciona aunque el JSONL original ya no exista en disco
