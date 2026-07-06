## MODIFIED Requirements

### Requirement: Presentación de novedades (What's New)

Cuando se detecta un cambio de versión, la app SHALL mostrar una única vez un modal "Novedades / What's New" con la sección del changelog correspondiente a la versión actual, extraída del `CHANGELOG.md` empaquetado. El contenido de la sección SHALL renderizarse como markdown (encabezados, listas, énfasis, enlaces), no como texto plano.

#### Scenario: Modal con las notas de la versión

- **WHEN** se detectó un cambio de versión y existe una sección de changelog para la versión actual
- **THEN** se muestra el modal con el título de la versión y su contenido (Added / Fixed / Changed…) renderizado como markdown legible (encabezados, listas con viñetas, énfasis y enlaces), sin mostrar marcadores markdown literales

#### Scenario: Sin sección de changelog para la versión

- **WHEN** se detectó un cambio de versión pero no hay una sección de changelog que coincida con la versión actual
- **THEN** no se muestra el modal (o se muestra un mensaje neutro), y de todas formas se marca la versión como vista para no reintentar en cada arranque

#### Scenario: Cierre marca la versión como vista

- **WHEN** el usuario cierra el modal de novedades
- **THEN** la versión actual queda persistida como "última versión vista" y el modal no vuelve a aparecer en arranques posteriores de la misma versión
