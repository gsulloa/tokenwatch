## Why

El costo de los modelos Sonnet 5 (`claude-sonnet-5`) — y de cualquier familia futura en su versión mayor 5 (Opus 5, Haiku 5) — se contabiliza como **$0**. El emparejador de precios (`price_row`) exige la subcadena `sonnet-4` / `opus-4` / `haiku-4`, atada a la versión mayor `4`; un id como `claude-sonnet-5` no coincide, cae en la rama de "modelo desconocido" y devuelve costo 0. Los tokens sí se registran, pero el gasto reportado queda subestimado.

## What Changes

- Cambiar el emparejamiento de precios para que resuelva por **familia de modelo** (`sonnet` / `opus` / `haiku`) en lugar de exigir la versión mayor `4`.
- `claude-sonnet-5`, `claude-opus-*` y `claude-haiku-*` de cualquier versión mayor resolverán a su fila de precios correspondiente.
- Ampliar la cobertura de tests para incluir ids de la generación 5 (`claude-sonnet-5`) y proteger contra la regresión.
- Nota operativa: el costo se persiste en el momento de la ingesta; los eventos ya almacenados con costo 0 no se recalculan con este cambio (solo aplica a ingestas nuevas). Se documenta como limitación conocida.

## Capabilities

### New Capabilities
<!-- Ninguna capacidad nueva; es un arreglo de comportamiento en una capacidad existente. -->

### Modified Capabilities
- `usage-ingestion`: el requisito "Cálculo de costo estimado" se refuerza para que el emparejamiento de modelo a tabla de precios resuelva por familia y no dependa de la versión mayor, de modo que nuevas versiones mayores de una familia conocida se contabilicen correctamente.

## Impact

- `packages/app/src-tauri/src/pricing.rs` — función `price_row` (lógica de emparejamiento) y su módulo de tests.
- Sin cambios de API, esquema de base de datos ni dependencias. El resto de la ingesta (`ingest/mod.rs`) permanece igual; solo cambia qué fila de precios devuelve `price_row`.
