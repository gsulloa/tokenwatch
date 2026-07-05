## MODIFIED Requirements

### Requirement: Consulta de series temporales agregadas
El sistema SHALL exponer un comando `query_series` que reciba una agrupación temporal (hora / día / semana / mes), una métrica (tokens totales / costo), un criterio de series (modelo / proyecto / modelo-proyecto) y un rango de fechas opcional (`since` / `until`), y devuelva las etiquetas de bucket ordenadas junto con una serie por grupo alineada a esos buckets. Los buckets temporales (hora/día/semana/mes) SHALL calcularse en **hora local** del sistema, aunque los eventos se almacenen y filtren en UTC. Los filtros `since` / `until` SHALL aceptar precisión de fecha y hora (datetime) para poder expresar rangos relativos exactos como las últimas 24 horas.

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

#### Scenario: Buckets vacíos rellenados
- **WHEN** un bucket dentro del rango no tiene eventos para una serie
- **THEN** ese punto se devuelve como 0 para mantener la línea continua

#### Scenario: Costo como métrica
- **WHEN** se consulta métrica=costo
- **THEN** cada punto es la suma del costo estimado de los eventos del bucket

#### Scenario: Series por modelo-proyecto
- **WHEN** se consulta series=modelo-proyecto
- **THEN** cada serie combina modelo y proyecto (p.ej. `claude-opus-4-8 · backend/madrid`)

### Requirement: Gráfico de área apilada con controles
La aplicación SHALL mostrar un gráfico de **área apilada (stacked area)** con controles segmentados —agrupación temporal (hora/día/semana/mes), métrica (tokens/costo) y series (modelo/proyecto/modelo-proyecto)— más un control de **rango de fechas** con presets recomendados, que actualicen el gráfico al cambiar. En cada punto del eje X las series SHALL acumularse (apilarse) de modo que la altura total del apilado represente el consumo agregado de ese bucket y cada banda represente el aporte de una serie. Las etiquetas del eje X SHALL mostrarse en hora local, incluyendo la hora del día cuando la granularidad es por hora.

#### Scenario: Cambio de control refresca el gráfico
- **WHEN** el usuario cambia cualquiera de los controles (rango, granularidad, métrica o series)
- **THEN** el gráfico se vuelve a consultar y renderiza con la nueva configuración

#### Scenario: Selección de preset de fecha
- **WHEN** el usuario elige un preset de rango (24h, 3 días, 7 días, 30 días, este mes o todo)
- **THEN** el gráfico consulta solo el rango correspondiente y el eje X refleja ese rango

#### Scenario: Rango personalizado
- **WHEN** el usuario elige "Custom" y define una fecha de inicio y fin
- **THEN** el gráfico consulta ese rango; los selectores están acotados por el rango real de datos disponibles

#### Scenario: Preset sugiere granularidad
- **WHEN** el usuario selecciona un preset de fecha
- **THEN** la granularidad se ajusta a un valor sugerido para ese rango (p.ej. 24h→hora, 30d→día), sin impedir que el usuario la cambie luego

#### Scenario: Guardarraíl de granularidad por hora
- **WHEN** el rango de fechas activo excede el límite recomendado para granularidad por hora (~3 días)
- **THEN** la opción "Hora" se deshabilita o se degrada a día para evitar cientos de barras ilegibles

#### Scenario: Etiquetas horarias en el eje X
- **WHEN** la granularidad activa es por hora
- **THEN** las etiquetas del eje X muestran la hora local (p.ej. `14:00`) además de la fecha cuando aplica

#### Scenario: Acumulado por punto en X
- **WHEN** un bucket del eje X tiene varias series con valores
- **THEN** las series se apilan una sobre otra y la altura total en ese punto equivale a la suma de todas las series del bucket

#### Scenario: Estado vacío
- **WHEN** no hay eventos que graficar en el rango seleccionado
- **THEN** la app muestra un estado vacío en lugar de un gráfico en blanco
