## MODIFIED Requirements

### Requirement: Medidores de sesión, semana y semana por modelo
El popover SHALL mostrar medidores de utilización para la **sesión (5h)** y la **semana** (total), cada uno con su porcentaje. Cada medidor SHALL mostrar una referencia temporal de su `resets_at` según el tiempo que falte: cuando falten **menos de 12 horas** MUST mostrar el **tiempo restante** ("resetea en Xh Ym"), y cuando falten **12 horas o más** MUST mostrar la **fecha de cierre** del período (día de la semana, día y mes, p.ej. "cierra el sáb 12 jul"). Este mismo umbral SHALL aplicarse a los medidores compactos semanales por modelo. Además, cuando el snapshot incluya límites **semanales por modelo**, el popover SHALL mostrar un medidor compacto por cada modelo, etiquetado con el nombre del modelo. Cuando no haya semanales por modelo, esa sección MUST omitirse. Cuando los datos de límites no estén disponibles, el popover MUST mostrar un estado explícito en lugar de porcentajes en blanco.

#### Scenario: Datos disponibles
- **WHEN** hay un snapshot de límites con sesión al 20 % y semana al 56 %
- **THEN** el popover muestra ambos porcentajes y una referencia temporal de reset para cada uno

#### Scenario: Semana con más de 12 horas hasta el reset
- **WHEN** el medidor semanal tiene un `resets_at` a 3 días vista (≥ 12 horas)
- **THEN** el medidor muestra la fecha de cierre del período (p.ej. "cierra el sáb 12 jul") en vez del tiempo restante en horas

#### Scenario: Semana con menos de 12 horas hasta el reset
- **WHEN** el medidor semanal tiene un `resets_at` a menos de 12 horas
- **THEN** el medidor muestra el tiempo restante ("resetea en Xh Ym")

#### Scenario: Sesión siempre en tiempo restante
- **WHEN** el medidor de sesión (5h) tiene su `resets_at` (siempre a menos de 12 horas)
- **THEN** el medidor muestra el tiempo restante ("resetea en Xh Ym")

#### Scenario: Semanales por modelo presentes
- **WHEN** el snapshot incluye semanales por modelo (p.ej. "Opus" al 40 %, "Fable" al 5 %)
- **THEN** el popover muestra un medidor compacto por cada modelo con su nombre, porcentaje y la referencia temporal de reset según el umbral de 12 horas

#### Scenario: Sin semanales por modelo
- **WHEN** el snapshot no incluye ningún semanal por modelo
- **THEN** el popover no muestra la sección por modelo

#### Scenario: Datos no disponibles
- **WHEN** los límites están no disponibles (p.ej. token expirado o no logueado)
- **THEN** el popover muestra un mensaje explicativo (p.ej. "Abre Claude Code para actualizar") en vez de porcentajes
