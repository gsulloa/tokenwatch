## MODIFIED Requirements

### Requirement: Gráfico de área apilada con controles
La aplicación SHALL mostrar un gráfico de **área apilada (stacked area)** con tres controles segmentados —agrupación temporal (día/semana/mes), métrica (tokens/costo) y series (modelo/proyecto/modelo-proyecto)— que actualicen el gráfico al cambiar. En cada punto del eje X las series SHALL acumularse (apilarse) de modo que la altura total del apilado represente el consumo agregado de ese bucket y cada banda represente el aporte de una serie.

#### Scenario: Cambio de control refresca el gráfico
- **WHEN** el usuario cambia cualquiera de los tres controles
- **THEN** el gráfico se vuelve a consultar y renderiza con la nueva configuración

#### Scenario: Acumulado por punto en X
- **WHEN** un bucket del eje X tiene varias series con valores
- **THEN** las series se apilan una sobre otra y la altura total en ese punto equivale a la suma de todas las series del bucket

#### Scenario: Estado vacío
- **WHEN** no hay eventos que graficar
- **THEN** la app muestra un estado vacío en lugar de un gráfico en blanco

## ADDED Requirements

### Requirement: Tabla de datos con cifras exactas
La aplicación SHALL mostrar, debajo del gráfico, una tabla con una fila por serie y una columna por bucket, presentando las cifras exactas de la métrica seleccionada (tokens o costo). La tabla SHALL incluir una columna de total por serie y una fila de total por bucket, y SHALL respetar el mismo orden y color de series que el gráfico.

#### Scenario: Lectura de cifras exactas
- **WHEN** el usuario mira la tabla
- **THEN** cada celda muestra el valor exacto de la serie en ese bucket para la métrica activa (tokens sin abreviar o costo con decimales)

#### Scenario: Totales por serie y por bucket
- **WHEN** hay más de una serie o más de un bucket
- **THEN** la tabla muestra un total por cada serie (columna) y un total por cada bucket (fila)

#### Scenario: Consistencia con el gráfico
- **WHEN** el gráfico muestra un color y orden para una serie
- **THEN** la fila correspondiente en la tabla usa el mismo color de identificación y el mismo orden

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

## REMOVED Requirements

### Requirement: Limitar series visibles con agrupación de "otros"
**Reason**: El usuario necesita ver todas las series por separado para no perder el detalle por proyecto/modelo; agrupar en "otros" oculta información justo cuando hay muchas series.
**Migration**: Todas las series se renderizan individualmente en el gráfico apilado y en la tabla. No existe reemplazo del bucket "otros"; el detalle completo queda disponible en la tabla de cifras exactas.
