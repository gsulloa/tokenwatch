## MODIFIED Requirements

### Requirement: Popover desplegable desde el tray
La aplicación SHALL mostrar una ventana popover sin bordes al hacer click en el icono del tray, posicionada junto al icono, y SHALL ocultarla cuando pierde el foco. Al arrancar, la ventana MUST estar oculta (no visible como ventana normal).

El popover MUST poder mostrarse sobre el Space actualmente activo, incluidas las ventanas en modo pantalla completa, sin requerir que el usuario cambie de Space. Esto aplica también cuando la ventana en pantalla completa pertenece a **otra** aplicación. Para ello, en macOS la ventana SHALL configurar un `collectionBehavior` que le permita unirse a todos los Spaces y actuar como auxiliar de pantalla completa, y un nivel de ventana suficiente para aparecer por encima de las ventanas en fullscreen.

El popover MUST mostrarse desde el primer click tras arrancar, aunque la app nunca haya sido activada (p.ej. sin haber abierto el dashboard), y MUST hacerlo sin activar la app. Para ello:
- el panel MUST NOT ocultarse al desactivarse la app;
- el panel SHALL conservar los bits de estilo existentes al añadir el estilo no-activante;
- el estilo no-activante SHALL aplicarse cuando la ventana ya es un `NSPanel`, es decir, después de convertirla, de modo que el panel sea realmente no-activante y no solo tenga el bit marcado en la máscara;
- mostrar el popover MUST NOT activar la aplicación ni provocar un cambio de Space, aunque el foco esté en otra app a pantalla completa.

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

#### Scenario: Mostrar sobre la pantalla completa de otra app
- **WHEN** otra aplicación (p.ej. Safari, Terminal o VS Code) está en pantalla completa nativa en su propio Space, tiene el foco, y el usuario hace click en el icono de TokenWatch en la barra de menú
- **THEN** el popover se muestra bajo el icono, por encima de esa app y en ese mismo Space, sin animación de cambio de Space, sin que TokenWatch pase a ser la app activa y sin que aparezca icono en el Dock

#### Scenario: Cerrar con un segundo click sobre pantalla completa
- **WHEN** el popover está visible sobre la pantalla completa de otra app y el usuario vuelve a hacer click en el icono del tray
- **THEN** el popover se oculta, no se reabre en el mismo click y el usuario sigue en el Space de pantalla completa

#### Scenario: Ocultar al hacer click fuera sobre pantalla completa
- **WHEN** el popover está visible sobre la pantalla completa de otra app y el usuario hace click en el contenido de esa app
- **THEN** el popover se oculta y la app en pantalla completa conserva el foco y su Space

#### Scenario: Pantalla completa en una pantalla secundaria
- **WHEN** hay varias pantallas con Spaces separados, otra app está en pantalla completa en la pantalla secundaria y el usuario hace click en el icono del tray de la barra de menú de esa pantalla
- **THEN** el popover se muestra en esa pantalla, bajo el icono y dentro de su área visible, sobre la app en pantalla completa y sin cambiar de Space en ninguna pantalla

#### Scenario: Ocultar al perder foco
- **WHEN** el popover está visible y el usuario hace click fuera de él
- **THEN** el popover se oculta

#### Scenario: Oculto al arrancar
- **WHEN** la aplicación arranca
- **THEN** no se muestra ninguna ventana hasta que el usuario hace click en el tray

#### Scenario: Estado guardado del dashboard en pantalla completa
- **WHEN** el dashboard se cerró estando en pantalla completa y la app se reinicia
- **THEN** al arrancar no se restaura la pantalla completa ni se muestra el dashboard, y el primer click en el tray muestra el popover
