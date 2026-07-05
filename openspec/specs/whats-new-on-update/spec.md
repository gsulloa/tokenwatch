# whats-new-on-update Specification

## Purpose

Detectar cambios de versión al arrancar la app y mostrar una única vez un modal "Novedades / What's New" con la sección del changelog correspondiente a la versión actual.

## Requirements

### Requirement: Detección de cambio de versión

La app SHALL detectar, al arrancar, si la versión en ejecución difiere de la última versión que el usuario ya vio, persistiendo la "última versión vista".

#### Scenario: Primera ejecución de una versión nueva

- **WHEN** la app arranca y la versión en runtime no coincide con la "última versión vista" persistida
- **THEN** se considera que hubo un cambio de versión y se marca para mostrar las novedades de esa versión

#### Scenario: Versión ya vista

- **WHEN** la app arranca y la versión en runtime coincide con la "última versión vista" persistida
- **THEN** no se muestran novedades

#### Scenario: Primera instalación (sin versión previa)

- **WHEN** la app arranca por primera vez y no existe ninguna "última versión vista" persistida
- **THEN** la app registra la versión actual como vista y NO muestra el modal de novedades (evita ruido en la primera instalación)

### Requirement: Presentación de novedades (What's New)

Cuando se detecta un cambio de versión, la app SHALL mostrar una única vez un modal "Novedades / What's New" con la sección del changelog correspondiente a la versión actual, extraída del `CHANGELOG.md` empaquetado.

#### Scenario: Modal con las notas de la versión

- **WHEN** se detectó un cambio de versión y existe una sección de changelog para la versión actual
- **THEN** se muestra el modal con el título de la versión y su contenido (Added / Fixed / Changed…) renderizado de forma legible

#### Scenario: Sin sección de changelog para la versión

- **WHEN** se detectó un cambio de versión pero no hay una sección de changelog que coincida con la versión actual
- **THEN** no se muestra el modal (o se muestra un mensaje neutro), y de todas formas se marca la versión como vista para no reintentar en cada arranque

#### Scenario: Cierre marca la versión como vista

- **WHEN** el usuario cierra el modal de novedades
- **THEN** la versión actual queda persistida como "última versión vista" y el modal no vuelve a aparecer en arranques posteriores de la misma versión
