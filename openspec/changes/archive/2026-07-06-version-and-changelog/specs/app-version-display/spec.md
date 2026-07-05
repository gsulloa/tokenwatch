## ADDED Requirements

### Requirement: Mostrar la versión actual en runtime

La app SHALL exponer una superficie "Acerca de / Versión" que muestre la versión actual de TokenWatch obtenida en tiempo de ejecución (no un valor hardcodeado en el frontend), con el formato `vX.Y.Z`.

#### Scenario: Versión visible en la superficie Acerca de

- **WHEN** el usuario abre la superficie "Acerca de / Versión"
- **THEN** se muestra la versión actual de la app como `vX.Y.Z`, obtenida de la API de Tauri en runtime

#### Scenario: Entorno no-Tauri (dev en browser)

- **WHEN** la app corre fuera de Tauri (por ejemplo `vite dev` en el navegador) y la versión en runtime no está disponible
- **THEN** la superficie no rompe: muestra un placeholder degradado (por ejemplo `v—` o "dev") sin lanzar errores

### Requirement: Acción de buscar/instalar actualización desde Acerca de

La superficie "Acerca de / Versión" SHALL ofrecer una acción explícita para buscar actualizaciones y, cuando exista una, instalarla, reutilizando el flujo de actualización existente (`useAppUpdate`).

#### Scenario: Buscar actualizaciones manualmente

- **WHEN** el usuario activa la acción "Buscar actualizaciones" en la superficie Acerca de
- **THEN** se dispara un chequeo de actualización y la UI refleja el estado (`checking`, `available`, `downloading`, `ready`, `error`) según el flujo existente

#### Scenario: App al día

- **WHEN** el chequeo termina y no hay una versión más nueva disponible
- **THEN** la superficie indica que la app está actualizada, sin ofrecer un botón de instalación

### Requirement: Acceso al changelog completo

La superficie "Acerca de / Versión" SHALL ofrecer una forma de ver el changelog completo de la app a partir del `CHANGELOG.md` empaquetado.

#### Scenario: Ver changelog completo

- **WHEN** el usuario elige "Ver changelog" (o equivalente) desde la superficie Acerca de
- **THEN** se muestra el contenido del changelog empaquetado (`src/generated/changelog.md`) renderizado de forma legible
