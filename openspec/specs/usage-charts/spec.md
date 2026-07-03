# usage-charts Specification

## Purpose
TBD - created by archiving change usage-charts. Update Purpose after archive.
## Requirements
### Requirement: Consulta de series temporales agregadas
El sistema SHALL exponer un comando `query_series` que reciba una agrupación temporal (día / semana / mes), una métrica (tokens totales / costo) y un criterio de series (modelo / proyecto / modelo-proyecto), y devuelva las etiquetas de bucket ordenadas junto con una serie por grupo alineada a esos buckets.

#### Scenario: Agregación por día y modelo
- **WHEN** se consulta bucket=día, métrica=tokens, series=modelo
- **THEN** se devuelve una serie por modelo con la suma de tokens por día

#### Scenario: Buckets vacíos rellenados
- **WHEN** un bucket dentro del rango no tiene eventos para una serie
- **THEN** ese punto se devuelve como 0 para mantener la línea continua

#### Scenario: Costo como métrica
- **WHEN** se consulta métrica=costo
- **THEN** cada punto es la suma del costo estimado de los eventos del bucket

#### Scenario: Series por modelo-proyecto
- **WHEN** se consulta series=modelo-proyecto
- **THEN** cada serie combina modelo y proyecto (p.ej. `claude-opus-4-8 · backend/madrid`)

### Requirement: Gráfico de línea con controles
La aplicación SHALL mostrar un gráfico de línea con tres controles segmentados —agrupación temporal (día/semana/mes), métrica (tokens/costo) y series (modelo/proyecto/modelo-proyecto)— que actualicen el gráfico al cambiar.

#### Scenario: Cambio de control refresca el gráfico
- **WHEN** el usuario cambia cualquiera de los tres controles
- **THEN** el gráfico se vuelve a consultar y renderiza con la nueva configuración

#### Scenario: Estado vacío
- **WHEN** no hay eventos que graficar
- **THEN** la app muestra un estado vacío en lugar de un gráfico en blanco

### Requirement: Costo mostrado como estimado
La aplicación SHALL indicar de forma visible que los valores de costo son estimados.

#### Scenario: Nota de estimación visible
- **WHEN** la métrica seleccionada es costo
- **THEN** la UI muestra una nota de que el costo es estimado

### Requirement: Limitar series visibles con agrupación de "otros"
Cuando el número de series supere un máximo (top-N por total), la aplicación SHALL mostrar solo las top-N y agrupar el resto en una serie "otros", indicando cuántas series se agruparon.

#### Scenario: Muchos proyectos
- **WHEN** hay más series que el máximo configurado
- **THEN** se muestran las top-N y una serie "otros" con el resto, indicando cuántas se agruparon

### Requirement: Refresco automático ante nuevos datos
La aplicación SHALL escuchar el evento `usage-updated` y volver a consultar las series cuando llegue, además de mostrar la marca de tiempo del último refresh.

#### Scenario: Llega usage-updated
- **WHEN** el backend emite `usage-updated`
- **THEN** el frontend re-consulta y actualiza el gráfico y el último refresh

