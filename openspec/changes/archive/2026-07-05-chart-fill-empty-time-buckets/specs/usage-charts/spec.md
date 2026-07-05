## MODIFIED Requirements

### Requirement: Consulta de series temporales agregadas
El sistema SHALL exponer un comando `query_series` que reciba una agrupación temporal (hora / día / semana / mes), una métrica (tokens totales / costo), un criterio de series (modelo / proyecto / modelo-proyecto) y un rango de fechas opcional (`since` / `until`), y devuelva las etiquetas de bucket ordenadas junto con una serie por grupo alineada a esos buckets. Los buckets temporales (hora/día/semana/mes) SHALL calcularse en **hora local** del sistema, aunque los eventos se almacenen y filtren en UTC. Los filtros `since` / `until` SHALL aceptar precisión de fecha y hora (datetime) para poder expresar rangos relativos exactos como las últimas 24 horas.

La respuesta SHALL incluir **todos** los buckets que caben dentro del rango efectivo según la granularidad, no solo aquellos que tienen eventos. El sistema SHALL enumerar la secuencia completa de buckets locales desde el inicio hasta el fin del rango (con el paso propio de la granularidad: hora/día/semana/mes) y alinear cada serie contra esa secuencia. Cuando `since`/`until` no se especifican, el rango efectivo SHALL derivarse del timestamp mínimo y máximo de los eventos disponibles, y los buckets intermedios sin eventos SHALL rellenarse igualmente. Los buckets sin eventos para una serie SHALL devolverse en 0 para mantener el eje X completo y la línea continua.

#### Scenario: Agregación por día y modelo
- **WHEN** se consulta bucket=día, métrica=tokens, series=modelo
- **THEN** se devuelve una serie por modelo con la suma de tokens por día

#### Scenario: Agregación por hora
- **WHEN** se consulta bucket=hora
- **THEN** cada etiqueta de bucket corresponde a una hora local y agrupa los eventos ocurridos dentro de esa hora local

#### Scenario: Bucket en hora local
- **WHEN** un evento ocurrió a las 21:00 en hora local (guardado en UTC)
- **THEN** ese evento se agrupa en el bucket de las 21:00 local, no en la hora UTC equivalente

#### Scenario: Filtro por rango de fechas
- **WHEN** se consulta con `since` y/o `until`
- **THEN** solo se incluyen eventos cuyo timestamp UTC cae dentro del rango, comparado con la precisión datetime recibida

#### Scenario: Rango relativo de últimas 24 horas
- **WHEN** el rango pedido son las últimas 24 horas contadas desde el momento actual
- **THEN** solo se incluyen eventos de las últimas 24 horas, con precisión de hora (no truncado a inicio del día)

#### Scenario: Rango completo de buckets sin eventos intermedios
- **WHEN** se consulta bucket=hora para las últimas 24 horas y solo hay eventos en algunas horas
- **THEN** la respuesta contiene un bucket por cada hora del rango (las 24 horas), y las horas sin eventos aparecen con valor 0 en todas las series

#### Scenario: Buckets vacíos rellenados
- **WHEN** un bucket dentro del rango no tiene eventos para una serie
- **THEN** ese punto se devuelve como 0 para mantener la línea continua

#### Scenario: Bucket final incluido
- **WHEN** el rango efectivo termina en un instante que cae dentro de un bucket
- **THEN** ese bucket final se incluye en la secuencia (el fin del rango es inclusivo respecto al bucket que lo contiene)

#### Scenario: Costo como métrica
- **WHEN** se consulta métrica=costo
- **THEN** cada punto es la suma del costo estimado de los eventos del bucket

#### Scenario: Series por modelo-proyecto
- **WHEN** se consulta series=modelo-proyecto
- **THEN** cada serie combina modelo y proyecto (p.ej. `claude-opus-4-8 · backend/madrid`)
