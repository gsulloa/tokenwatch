# menubar-badge Specification

## Purpose
TBD - created by archiving change menubar-usage-badge. Update Purpose after archive.
## Requirements
### Requirement: Etiqueta de uso en la barra de menú
La aplicación SHALL poder mostrar el porcentaje de uso como **texto junto al
icono del tray** en la barra de menú de macOS, usando `TrayIcon::set_title`. El
texto MUST ser compacto (formato `"N%"`, sin decimales). El `TrayIcon` SHALL
conservarse en el estado de la app para poder actualizar el título después del
arranque sin recrear el icono ni perder su posición. El modo `off` MUST dejar el
ítem **solo con icono** (`set_title(None)`), que es el comportamiento por defecto.

#### Scenario: Etiqueta visible con datos
- **WHEN** el modo de la etiqueta es `session` y el snapshot indica 45 % de sesión
- **THEN** junto al icono del tray aparece el texto `45%`

#### Scenario: Modo solo icono por defecto
- **WHEN** el modo de la etiqueta es `off`
- **THEN** el ítem de la barra de menú muestra únicamente el icono, sin texto

#### Scenario: Se conserva la posición al actualizar
- **WHEN** el título cambia porque llega un snapshot nuevo
- **THEN** el mismo ítem del tray actualiza su texto en sitio, sin recrearse ni
  cambiar de posición

### Requirement: Contenido configurable de la etiqueta
La aplicación SHALL permitir configurar qué muestra la etiqueta entre los modos
`off`, `session` (porcentaje de la sesión de 5h), `week` (porcentaje de la semana)
y `max` (el mayor entre sesión y semana). El modo SHALL persistir en la tabla
`meta` bajo una clave dedicada y su valor por defecto MUST ser `off`. El sistema
SHALL exponer comandos Tauri para leer y escribir el modo
(`get_menubar_badge_mode` / `set_menubar_badge_mode`). Al escribirse un nuevo
modo, la etiqueta MUST re-aplicarse inmediatamente con el último snapshot conocido.

#### Scenario: Cambiar de modo actualiza la etiqueta al instante
- **WHEN** el usuario cambia el modo de `off` a `week` y la semana está al 56 %
- **THEN** el ítem de la barra de menú pasa a mostrar `56%` sin esperar al
  siguiente ciclo de datos

#### Scenario: Modo `max` toma el mayor porcentaje
- **WHEN** el modo es `max`, la sesión está al 20 % y la semana al 56 %
- **THEN** la etiqueta muestra `56%`

#### Scenario: Persistencia entre reinicios
- **WHEN** el usuario selecciona `session` y luego reinicia la app
- **THEN** al arrancar la etiqueta vuelve a mostrarse en modo `session`

### Requirement: Actualización por datos de límites
La etiqueta SHALL actualizarse a partir del mismo flujo de datos que alimenta el
popover: SHALL calcularse con el último snapshot al construir el tray (si existe)
y SHALL recalcularse cuando el backend emite `limits-updated`. La etiqueta MUST
NOT introducir un ciclo de polling propio adicional.

#### Scenario: Actualización por evento
- **WHEN** el backend emite `limits-updated` con un snapshot nuevo y el modo no es
  `off`
- **THEN** el texto de la etiqueta se recalcula y se actualiza con el nuevo
  porcentaje

#### Scenario: Valor inicial al arrancar
- **WHEN** la app arranca con un modo distinto de `off` y ya existe un snapshot
  reciente
- **THEN** la etiqueta muestra el porcentaje correspondiente desde el inicio

### Requirement: Estado sin datos disponibles
La etiqueta SHALL mostrar un marcador no numérico (`–`) en lugar de un porcentaje
cuando los datos de límites **no** estén disponibles (p.ej. token expirado o sin
login) y el modo no sea `off`. La etiqueta MUST NOT conservar el último valor como
si fuera actual.

#### Scenario: Límites no disponibles
- **WHEN** el modo es `session` y el snapshot de límites no está disponible
- **THEN** la etiqueta muestra `–` en vez de un porcentaje

#### Scenario: No se muestran valores obsoletos
- **WHEN** los límites pasan de disponibles a no disponibles
- **THEN** la etiqueta reemplaza el porcentaje anterior por `–`

### Requirement: Orientación para priorizar el icono en macOS
La app SHALL ofrecer en el popover una ayuda breve que explique cómo reposicionar
el icono manualmente (⌘-arrastrar hacia la izquierda de la barra), dado que macOS
no permite a una app fijar el orden ni la prioridad de su ítem en la barra de
menú. Esta ayuda MUST NOT afirmar que la app puede forzar la visibilidad del ítem.

#### Scenario: Ayuda de reposicionamiento visible
- **WHEN** el usuario abre el popover y consulta las opciones de la etiqueta
- **THEN** se muestra una indicación de que puede arrastrar el icono con ⌘ para
  moverlo a una posición más visible

