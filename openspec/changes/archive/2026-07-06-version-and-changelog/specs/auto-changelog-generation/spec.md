## ADDED Requirements

### Requirement: Generar la sección Unreleased desde commits

El flujo de release SHALL contar con un script que genere/actualice la sección `## [Unreleased]` de `CHANGELOG.md` a partir de los commits con convención (Conventional Commits) desde el último tag de versión, sin edición manual.

#### Scenario: Commits con convención agrupados por tipo

- **WHEN** se ejecuta el generador y existen commits desde el último tag con prefijos `feat`, `fix`, `perf`, etc.
- **THEN** la sección `## [Unreleased]` se rellena con esos cambios agrupados por categoría Keep a Changelog (por ejemplo `feat` → Added, `fix` → Fixed, `perf`/`refactor` → Changed)

#### Scenario: Commits sin convención se omiten o agrupan

- **WHEN** existen commits que no siguen la convención (o de tipos ignorados como `chore`, `ci`, `docs` según configuración)
- **THEN** el generador los excluye de la lista de novedades del usuario (o los agrupa bajo una categoría explícita), sin romper el proceso

#### Scenario: Sin cambios relevantes

- **WHEN** no hay commits relevantes desde el último tag
- **THEN** la sección `## [Unreleased]` queda vacía (o con un marcador neutro) y el proceso termina sin error

### Requirement: Integración con el flujo de release existente

El generador de changelog SHALL integrarse en el flujo de release de modo que corra ANTES de que `bump-version.mjs` promueva `## [Unreleased]` a la versión fechada, preservando el formato Keep a Changelog y el modelo de fuente única (`CHANGELOG.md` en la raíz).

#### Scenario: Orden en el flujo de release

- **WHEN** se ejecuta el flujo de release (`release.sh` / workflow) para una nueva versión
- **THEN** primero se genera/actualiza `## [Unreleased]` con el script, y luego `bump-version.mjs` la promueve a `## [X.Y.Z] - <fecha>`

#### Scenario: Downstream alimentado por la misma fuente

- **WHEN** el release se publica tras generar el changelog automáticamente
- **THEN** las notas del release de GitHub, las notas del manifiesto del updater y el modal *What's New* de la app se alimentan de esa misma sección promovida del `CHANGELOG.md`, sin notas genéricas de fallback cuando hubo commits relevantes

#### Scenario: Formato Keep a Changelog preservado

- **WHEN** el generador escribe en `CHANGELOG.md`
- **THEN** respeta el formato Keep a Changelog (encabezado, `## [Unreleased]`, categorías) de modo que `promoteUnreleased` y `sync-changelog.mjs` sigan funcionando sin cambios en su contrato
