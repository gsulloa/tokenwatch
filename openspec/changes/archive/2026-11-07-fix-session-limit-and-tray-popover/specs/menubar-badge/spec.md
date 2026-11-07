## ADDED Requirements

### Requirement: Respaldo semanal en modo sesión
Cuando el modo de la etiqueta es `session` y el snapshot de límites está disponible pero no contiene una ventana de sesión activa, la etiqueta SHALL mostrar el porcentaje de la semana precedido del prefijo `W ` (formato `"W N%"`, sin decimales) en lugar de `–`. El prefijo sirve para distinguirlo del porcentaje de sesión. Los modos `week` y `max` MUST conservar el formato `"N%"` sin prefijo.

#### Scenario: Sin sesión activa con semana disponible
- **WHEN** el modo es `session`, el snapshot está disponible, no hay sesión y la semana está al 56 %
- **THEN** la etiqueta muestra `W 56%`

#### Scenario: Vuelve la sesión
- **WHEN** el modo es `session` y llega un snapshot con sesión al 12 %
- **THEN** la etiqueta muestra `12%` sin prefijo

#### Scenario: Modo max sin sesión
- **WHEN** el modo es `max`, no hay sesión y la semana está al 56 %
- **THEN** la etiqueta muestra `56%` sin prefijo

## MODIFIED Requirements

### Requirement: Estado sin datos disponibles
La etiqueta SHALL mostrar un marcador no numérico (`–`) en lugar de un porcentaje, siempre que el modo no sea `off`, en dos casos:
- los datos de límites **no** están disponibles (p.ej. token expirado o sin login);
- los datos están disponibles pero no existe ninguna ventana aplicable al modo, contando el respaldo semanal del modo `session`.

La etiqueta MUST NOT conservar el último valor como si fuera actual.

#### Scenario: Límites no disponibles
- **WHEN** el modo es `session` y el snapshot de límites no está disponible
- **THEN** la etiqueta muestra `–` en vez de un porcentaje

#### Scenario: Sin sesión ni semana
- **WHEN** el modo es `session` y el snapshot disponible no contiene ni sesión ni semana
- **THEN** la etiqueta muestra `–`

#### Scenario: No se muestran valores obsoletos
- **WHEN** los límites pasan de disponibles a no disponibles
- **THEN** la etiqueta reemplaza el porcentaje anterior por `–`
