## MODIFIED Requirements

### Requirement: Cálculo de costo estimado

El sistema SHALL calcular un costo estimado por evento como `input·Pin + output·Pout + cache_creation·Pwrite + cache_read·Pread`, usando una tabla de precios embebida por **familia de modelo**. La tabla SHALL cubrir, como mínimo, las familias que Claude Code emite hoy: el tier premium (`fable` / `mythos`), `opus`, `sonnet` y `haiku`.

El emparejamiento de un id de modelo a su fila de precios SHALL resolver por el nombre de familia y MUST NOT depender de la versión mayor del modelo, de modo que nuevas versiones mayores de una familia conocida (p. ej. `claude-sonnet-5`, `claude-opus-5`) se contabilicen con la fila de precios de esa familia. El emparejamiento MUST tolerar alias desnudos (`sonnet`, `opus`) y prefijos de proveedor (p. ej. `global.anthropic.claude-sonnet-4-6`).

Las familias `fable` y `mythos` SHALL compartir una única fila de precios, dado que Claude Fable 5 y Claude Mythos 5 tienen precios idénticos y difieren sólo en el canal de distribución. Esa fila SHALL ser más cara que la de `opus`, reflejando que el tier premium se tarifa por encima del tier Opus.

Un modelo cuya familia no esté en la tabla MUST resultar en costo 0 y registrar un log, sin interrumpir la ingesta. Una suite de tests SHALL verificar que toda familia que Claude Code emite resuelve a una fila de precios, de modo que la ausencia de una familia falle en CI en lugar de producir silenciosamente costo 0.

#### Scenario: Modelo de familia conocida, cualquier versión

- **WHEN** el evento usa un modelo cuya familia está en la tabla, en cualquier versión mayor (p. ej. `claude-sonnet-5`, `claude-opus-4-8`, `claude-haiku-4-5`)
- **THEN** el costo se calcula con las tarifas de esa familia

#### Scenario: Modelo del tier premium

- **WHEN** el evento usa `claude-fable-5`
- **THEN** el costo se calcula con la fila del tier premium, cuyas tarifas de input y output son mayores que las de la familia `opus`, y el costo resultante es mayor que cero

#### Scenario: Mythos comparte la fila de Fable

- **WHEN** el evento usa `claude-mythos-5` o `claude-mythos-preview`
- **THEN** el costo se calcula con exactamente la misma fila de precios que `claude-fable-5`

#### Scenario: Alias desnudo o prefijo de proveedor

- **WHEN** el evento usa un id sin versión (p. ej. `sonnet`) o con prefijo de proveedor (p. ej. `global.anthropic.claude-sonnet-4-6`)
- **THEN** el costo se calcula con las tarifas de la familia contenida en el id

#### Scenario: Modelo de familia desconocida

- **WHEN** el evento usa un modelo cuya familia no está en la tabla (p. ej. `gpt-4o`)
- **THEN** el costo es 0, se registra un log y la ingesta continúa

#### Scenario: Familia faltante detectada en CI

- **WHEN** una familia que Claude Code emite no tiene fila en la tabla embebida
- **THEN** la suite de tests falla, en lugar de que los eventos de esa familia se registren silenciosamente con costo 0
