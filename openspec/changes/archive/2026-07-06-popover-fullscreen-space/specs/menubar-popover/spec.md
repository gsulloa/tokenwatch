## MODIFIED Requirements

### Requirement: Popover desplegable desde el tray
La aplicación SHALL mostrar una ventana popover sin bordes al hacer click en el icono del tray, posicionada junto al icono, y SHALL ocultarla cuando pierde el foco. Al arrancar, la ventana MUST estar oculta (no visible como ventana normal). El popover MUST poder mostrarse sobre el Space actualmente activo, incluidas las ventanas en modo pantalla completa, sin requerir que el usuario cambie de Space; para ello la ventana SHALL configurar en macOS un `collectionBehavior` que le permita unirse a todos los Spaces y actuar como auxiliar de pantalla completa, y un nivel de ventana suficiente para aparecer por encima de las ventanas en fullscreen.

#### Scenario: Mostrar al hacer click
- **WHEN** el usuario hace click en el icono del tray y el popover está oculto
- **THEN** el popover se muestra junto al icono y toma el foco

#### Scenario: Mostrar sobre una ventana en pantalla completa
- **WHEN** el usuario está en un Space con una ventana en modo pantalla completa y hace click en el icono del tray
- **THEN** el popover se muestra sobre esa ventana en el Space activo, sin necesidad de cambiar al Space "escritorio"

#### Scenario: Ocultar al perder foco
- **WHEN** el popover está visible y el usuario hace click fuera de él
- **THEN** el popover se oculta

#### Scenario: Oculto al arrancar
- **WHEN** la aplicación arranca
- **THEN** no se muestra ninguna ventana hasta que el usuario hace click en el tray
