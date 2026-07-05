## MODIFIED Requirements

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
