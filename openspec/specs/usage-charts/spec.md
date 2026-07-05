# usage-charts Specification

## Purpose
TBD - created by archiving change usage-charts. Update Purpose after archive.
## Requirements
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

### Requirement: Tabla de datos con cifras exactas
La aplicación SHALL mostrar, debajo del gráfico, una tabla con una fila por serie y una columna por bucket, presentando las cifras exactas de la métrica seleccionada (tokens o costo). La tabla SHALL incluir una columna de total por serie y una fila de total por bucket, y SHALL respetar el mismo orden y color de series que el gráfico. La columna de nombre de serie/proyecto (la primera columna) SHALL permanecer fija durante el scroll horizontal de la tabla, tanto en el encabezado como en las filas de datos y en la fila de totales.

#### Scenario: Lectura de cifras exactas
- **WHEN** el usuario mira la tabla
- **THEN** cada celda muestra el valor exacto de la serie en ese bucket para la métrica activa (tokens sin abreviar o costo con decimales)

#### Scenario: Totales por serie y por bucket
- **WHEN** hay más de una serie o más de un bucket
- **THEN** la tabla muestra un total por cada serie (columna) y un total por cada bucket (fila)

#### Scenario: Consistencia con el gráfico
- **WHEN** el gráfico muestra un color y orden para una serie
- **THEN** la fila correspondiente en la tabla usa el mismo color de identificación y el mismo orden

#### Scenario: Columna de nombre fija al scrollear horizontalmente
- **WHEN** la tabla tiene más buckets de los que caben y el usuario hace scroll horizontal
- **THEN** la primera columna (nombre de serie/proyecto) permanece visible y fija a la izquierda mientras las columnas numéricas se desplazan por debajo, sin transparencias ni solapamientos ilegibles

### Requirement: Vista de dashboard con tarjetas de resumen
La aplicación SHALL presentar la vista de uso como un dashboard: una barra superior fija (nombre y subtítulo, marca de tiempo del último refresh y botón de actualizar), una fila de tarjetas KPI de resumen, una toolbar con los controles, y paneles tipo card para el gráfico y la tabla. Las tarjetas KPI SHALL mostrar al menos el total de la métrica seleccionada, el número de series, el número de períodos (buckets), el número de eventos y el rango de fechas de los datos.

#### Scenario: KPIs reflejan la selección actual
- **WHEN** el usuario cambia de métrica o de agrupación de series/período
- **THEN** las tarjetas de resumen se actualizan (p.ej. la tarjeta de total muestra "Total tokens" o "Costo total" según la métrica)

#### Scenario: Dashboard sin datos
- **WHEN** no hay datos de uso disponibles
- **THEN** las tarjetas muestran valores neutros ("—" o 0) y el panel del gráfico muestra el estado vacío, sin errores de render

### Requirement: Ventana redimensionable y pantalla completa
La aplicación SHALL abrirse en una ventana de tamaño de dashboard y permitir redimensionarla y llevarla a pantalla completa.

#### Scenario: Redimensionar y maximizar
- **WHEN** el usuario arrastra el borde de la ventana o usa el control de pantalla completa
- **THEN** la ventana se redimensiona y el dashboard se adapta de forma responsiva sin romper el layout

### Requirement: Tema de color según preferencia del sistema
La aplicación SHALL aplicar un tema de color (claro u oscuro) según la preferencia del sistema operativo, y SHALL reaccionar a los cambios de esa preferencia en caliente, de modo que todos los tokens de estilo estén definidos y la interfaz sea legible.

#### Scenario: Preferencia oscura del sistema
- **WHEN** el sistema está en modo oscuro
- **THEN** la aplicación se renderiza con el tema oscuro (fondos, bordes y texto legibles)

#### Scenario: Cambio de preferencia en caliente
- **WHEN** el usuario cambia la preferencia de color del sistema mientras la app está abierta
- **THEN** la aplicación actualiza el tema sin necesidad de reiniciarse

### Requirement: Costo mostrado como estimado
La aplicación SHALL indicar de forma visible que los valores de costo son estimados.

#### Scenario: Nota de estimación visible
- **WHEN** la métrica seleccionada es costo
- **THEN** la UI muestra una nota de que el costo es estimado

### Requirement: Refresco automático ante nuevos datos
La aplicación SHALL escuchar el evento `usage-updated` y volver a consultar las series cuando llegue, además de mostrar la marca de tiempo del último refresh.

#### Scenario: Llega usage-updated
- **WHEN** el backend emite `usage-updated`
- **THEN** el frontend re-consulta y actualiza el gráfico y el último refresh

