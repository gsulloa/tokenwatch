## MODIFIED Requirements

### Requirement: Cálculo de costo estimado
El sistema SHALL calcular un costo estimado por evento como `input·Pin + output·Pout + cache_creation·Pwrite + cache_read·Pread`, usando una tabla de precios embebida por **familia de modelo** (Opus / Sonnet / Haiku). El emparejamiento de un id de modelo a su fila de precios SHALL resolver por el nombre de familia (`opus` / `sonnet` / `haiku`) y MUST NOT depender de la versión mayor del modelo, de modo que nuevas versiones mayores de una familia conocida (p. ej. `claude-sonnet-5`) se contabilicen con la fila de precios de esa familia. Un modelo cuya familia no esté en la tabla MUST resultar en costo 0 y registrar un log, sin interrumpir la ingesta.

#### Scenario: Modelo de familia conocida, cualquier versión
- **WHEN** el evento usa un modelo cuya familia (`opus`, `sonnet` o `haiku`) está en la tabla, en cualquier versión mayor (p. ej. `claude-sonnet-5`, `claude-opus-4-8`, `claude-haiku-4-5`)
- **THEN** el costo se calcula con las tarifas de esa familia

#### Scenario: Modelo de familia desconocida
- **WHEN** el evento usa un modelo cuya familia no está en la tabla (p. ej. `gpt-4o`)
- **THEN** el costo es 0, se registra un log y la ingesta continúa
