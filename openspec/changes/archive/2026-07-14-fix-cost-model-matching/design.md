## Context

El costo por evento se calcula en Rust, en el momento de la ingesta, mediante una tabla de precios embebida (`packages/app/src-tauri/src/pricing.rs`). La función `price_row(model: &str)` decide qué fila de precios aplica buscando subcadenas fijas:

```rust
if model.contains("opus-4") { Some(OPUS) }
else if model.contains("sonnet-4") { Some(SONNET) }
else if model.contains("haiku-4") { Some(HAIKU) }
else { None }
```

Cada familia (Opus / Sonnet / Haiku) comparte una única fila de precios; el nombre no distingue versión mayor. Sin embargo, el emparejador ata la coincidencia a la versión mayor `4`. Con la llegada de `claude-sonnet-5`, ningún patrón coincide, `price_row` devuelve `None`, `cost()` registra `"unknown model — cost set to 0"` y devuelve `0.0`. Los tokens se persisten, pero el costo del evento queda en 0.

El costo se persiste en la fila del evento (`ingest/mod.rs`), no se recalcula al leer. Por eso el arreglo solo afecta ingestas nuevas.

## Goals / Non-Goals

**Goals:**
- Que `price_row` resuelva por familia de modelo (`opus` / `sonnet` / `haiku`) sin depender de la versión mayor, de modo que `claude-sonnet-5` y futuras versiones mayores de familias conocidas se contabilicen con su fila de precios.
- Mantener el comportamiento actual para modelos verdaderamente desconocidos (costo 0 + log).
- Cubrir con tests los ids de la generación 5.

**Non-Goals:**
- Diferenciar precios por versión mayor (hoy todas las versiones de una familia comparten fila; el precio de Sonnet 5 estándar coincide con la fila `SONNET` existente: $3/$15 in/out). No se introduce una tabla por-versión.
- Recalcular retroactivamente eventos ya persistidos con costo 0.
- Aplicar la tarifa introductoria promocional de Sonnet 5; se usa la tarifa estándar embebida.

## Decisions

**Decisión: emparejar por familia (`contains("sonnet")`) en lugar de por versión (`contains("sonnet-4")`).**
- Rationale: las filas de precios ya son por familia; atar la coincidencia a `-4` fue el defecto. Coincidir por el nombre de familia hace que cualquier versión mayor futura resuelva automáticamente, que es el patrón deseado y el que el comentario del código ("so minor version suffixes still resolve") ya pretendía.
- Alternativa considerada — añadir explícitamente `sonnet-5`/`opus-5`/`haiku-5`: rechazada porque repite el mismo error un ciclo más tarde (Sonnet 6 volvería a fallar). El emparejamiento por familia es estable a futuro.
- Alternativa considerada — tabla explícita id→precio con lista cerrada: rechazada por ahora; aumenta el mantenimiento y no aporta valor mientras todas las versiones de una familia compartan precio. Se deja como posible evolución si en el futuro los precios divergen por versión.
- Orden de evaluación: se mantiene el `if/else` por familia. Los nombres de familia (`opus`, `sonnet`, `haiku`) son mutuamente excluyentes en los ids de Claude, así que el orden no genera ambigüedad.

## Risks / Trade-offs

- [Un id no-Claude que contenga por casualidad "sonnet"/"opus"/"haiku" recibiría una fila de precios errónea] → Riesgo bajo: los ids provienen de `message.model` de archivos de Claude; el conjunto real son ids `claude-*`. Se mantiene la rama `None` para todo lo demás (p. ej. `gpt-4o` sigue devolviendo 0).
- [Eventos históricos ya persistidos con costo 0 no se corrigen] → Fuera de alcance; documentado como limitación conocida. Si se requiere, una re-ingesta o migración de recálculo sería un cambio aparte.
- [La fila `SONNET` usa la tarifa estándar, no la introductoria] → Aceptado: la promoción es temporal; el modelo de precios embebido asume tarifa estándar y ya lo hacía para el resto de familias.

## Migration Plan

- Cambio de una sola función más tests; no hay migración de datos ni de esquema.
- Rollback: revertir el commit restaura el emparejamiento por versión.
