## MODIFIED Requirements

### Requirement: Popover desplegable desde el tray
La aplicación SHALL mostrar una ventana popover sin bordes al hacer click en el icono del tray, posicionada junto al icono, y SHALL ocultarla cuando pierde el foco. Al arrancar, la ventana MUST estar oculta (no visible como ventana normal).

El popover MUST poder mostrarse sobre el Space actualmente activo, incluidas las ventanas en modo pantalla completa, sin requerir que el usuario cambie de Space. Para ello, en macOS la ventana SHALL configurar un `collectionBehavior` que le permita unirse a todos los Spaces y actuar como auxiliar de pantalla completa, y un nivel de ventana suficiente para aparecer por encima de las ventanas en fullscreen.

El popover MUST mostrarse desde el primer click tras arrancar, aunque la app nunca haya sido activada (p.ej. sin haber abierto el dashboard), y MUST hacerlo sin activar la app. Para ello:
- el panel MUST NOT ocultarse al desactivarse la app;
- el panel SHALL conservar los bits de estilo existentes al añadir el estilo no-activante.

Una pérdida de foco ocurrida dentro de un breve periodo de gracia tras mostrar el popover MUST NOT ocultarlo.

La persistencia del estado de ventanas MUST NOT aplicarse al popover. Para el dashboard, MUST NOT restaurar ni la pantalla completa ni la visibilidad.

#### Scenario: Mostrar al hacer click
- **WHEN** el usuario hace click en el icono del tray y el popover está oculto
- **THEN** el popover se muestra junto al icono y toma el foco

#### Scenario: Primer click sin haber abierto el dashboard
- **WHEN** la app acaba de arrancar, nunca se ha abierto el dashboard y el usuario hace click en el icono del tray
- **THEN** el popover se muestra junto al icono sin activar la app ni mostrar icono en el Dock

#### Scenario: Pérdida de foco espuria al mostrar
- **WHEN** el popover pierde el foco dentro del periodo de gracia inmediatamente posterior a mostrarse
- **THEN** el popover permanece visible y recupera el foco

#### Scenario: Click en el icono con el popover abierto
- **WHEN** el popover está visible y el usuario hace click en el icono del tray
- **THEN** el popover se oculta y no se vuelve a abrir en el mismo click

#### Scenario: Mostrar sobre una ventana en pantalla completa
- **WHEN** el usuario está en un Space con una ventana en modo pantalla completa y hace click en el icono del tray
- **THEN** el popover se muestra sobre esa ventana en el Space activo, sin necesidad de cambiar al Space "escritorio"

#### Scenario: Ocultar al perder foco
- **WHEN** el popover está visible y el usuario hace click fuera de él
- **THEN** el popover se oculta

#### Scenario: Oculto al arrancar
- **WHEN** la aplicación arranca
- **THEN** no se muestra ninguna ventana hasta que el usuario hace click en el tray

#### Scenario: Estado guardado del dashboard en pantalla completa
- **WHEN** el dashboard se cerró estando en pantalla completa y la app se reinicia
- **THEN** al arrancar no se restaura la pantalla completa ni se muestra el dashboard, y el primer click en el tray muestra el popover

### Requirement: Medidores de sesión, semana y semana por modelo
El popover SHALL mostrar medidores de utilización para la **sesión (5h)** y la **semana** (total), cada uno con su porcentaje.

Cada medidor SHALL mostrar una referencia temporal de su `resets_at` según el tiempo que falte:
- cuando falten **menos de 12 horas**, MUST mostrar el **tiempo restante** ("resetea en Xh Ym");
- cuando falten **12 horas o más**, MUST mostrar la **fecha de cierre** del período (día de la semana, día y mes, p.ej. "cierra el sáb 12 jul").

Este mismo umbral SHALL aplicarse a los medidores compactos semanales por modelo.

Cuando el snapshot incluya límites **semanales por modelo**, el popover SHALL mostrar un medidor compacto por cada modelo, etiquetado con el nombre del modelo. Cuando no haya semanales por modelo, esa sección MUST omitirse.

Cuando los datos de límites no estén disponibles, el popover MUST mostrar un estado explícito en lugar de porcentajes en blanco. Cuando el snapshot esté disponible e incluya la semana pero no una sesión activa, el popover MUST mostrar la fila del medidor de sesión con el texto "Sin sesión activa" en lugar de omitirla.

#### Scenario: Datos disponibles
- **WHEN** hay un snapshot de límites con sesión al 20 % y semana al 56 %
- **THEN** el popover muestra ambos porcentajes y una referencia temporal de reset para cada uno

#### Scenario: Sin sesión activa
- **WHEN** hay un snapshot de límites disponible con semana al 56 % y sin sesión
- **THEN** el popover muestra la fila "Sesión 5h" con el texto "Sin sesión activa" y el medidor semanal al 56 %

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

#### Scenario: Actualización por evento
- **WHEN** el backend emite `limits-updated`
- **THEN** los medidores se actualizan con el nuevo snapshot
