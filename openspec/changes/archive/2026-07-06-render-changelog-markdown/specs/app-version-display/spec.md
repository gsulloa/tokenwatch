## MODIFIED Requirements

### Requirement: Acceso al changelog completo

La superficie "Acerca de / Versión" SHALL ofrecer una forma de ver el changelog completo de la app a partir del `CHANGELOG.md` empaquetado. El changelog completo SHALL renderizarse en la ventana principal (dashboard), no en el popover del menu-bar, porque el popover es una ventana pequeña de tamaño fijo donde el contenido no se alcanza a ver. El contenido SHALL renderizarse como markdown (encabezados, listas, énfasis, enlaces), no como texto plano.

#### Scenario: Ver changelog completo desde el dashboard

- **WHEN** el usuario elige "Ver changelog" (o equivalente) en la ventana principal
- **THEN** se muestra el contenido del changelog empaquetado (`src/generated/changelog.md`) renderizado como markdown legible en esa ventana, sin mostrar marcadores markdown literales

#### Scenario: Ver changelog desde el popover

- **WHEN** el usuario elige "Ver changelog" en el popover del menu-bar
- **THEN** se abre/enfoca la ventana principal y el changelog completo se muestra ahí (no dentro del popover)
