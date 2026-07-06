# menubar-popover Specification

## Purpose
TBD - created by archiving change menubar-limits-popover. Update Purpose after archive.
## Requirements
### Requirement: Icono persistente en la barra de menú
La aplicación SHALL registrar un icono persistente en la barra de menú de macOS mediante `TrayIconBuilder` y SHALL fijar la política de activación a `Accessory`, de modo que la app no aparezca en el Dock. El icono MUST permanecer mientras la app esté en ejecución.

#### Scenario: Icono presente al arrancar
- **WHEN** la aplicación arranca
- **THEN** aparece un icono en la barra de menú y la app no muestra icono en el Dock

#### Scenario: Menú del tray
- **WHEN** el usuario abre el menú contextual del icono del tray
- **THEN** se ofrecen al menos las acciones "Abrir" y "Salir"

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

### Requirement: Medidores de sesión, semana y semana por modelo
El popover SHALL mostrar medidores de utilización para la **sesión (5h)** y la **semana** (total), cada uno con su porcentaje. Cada medidor SHALL mostrar una referencia temporal de su `resets_at` según el tiempo que falte: cuando falten **menos de 12 horas** MUST mostrar el **tiempo restante** ("resetea en Xh Ym"), y cuando falten **12 horas o más** MUST mostrar la **fecha de cierre** del período (día de la semana, día y mes, p.ej. "cierra el sáb 12 jul"). Este mismo umbral SHALL aplicarse a los medidores compactos semanales por modelo. Además, cuando el snapshot incluya límites **semanales por modelo**, el popover SHALL mostrar un medidor compacto por cada modelo, etiquetado con el nombre del modelo. Cuando no haya semanales por modelo, esa sección MUST omitirse. Cuando los datos de límites no estén disponibles, el popover MUST mostrar un estado explícito en lugar de porcentajes en blanco.

#### Scenario: Datos disponibles
- **WHEN** hay un snapshot de límites con sesión al 20 % y semana al 56 %
- **THEN** el popover muestra ambos porcentajes y una referencia temporal de reset para cada uno

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

### Requirement: Consumo por proyecto del día
El popover SHALL mostrar el consumo del día actual desglosado por proyecto, indicando para cada proyecto sus tokens y su porcentaje respecto al total del día, además del total global del día. El sistema SHALL exponer un comando `query_today_by_project` que agregue `usage_events` del día actual (en hora local) agrupando por `project_name`. Los porcentajes MUST sumar 100 % (salvo redondeo) y el total MUST coincidir con la suma de tokens de los proyectos listados.

#### Scenario: Varios proyectos en el día
- **WHEN** hoy hubo consumo en `tub2` y `argus`
- **THEN** el popover lista cada proyecto con sus tokens y su % del total del día, ordenados de mayor a menor, más el total del día

#### Scenario: Consistencia del total
- **WHEN** se listan los proyectos del día
- **THEN** la suma de los tokens de los proyectos es igual al total del día mostrado

#### Scenario: Día sin consumo
- **WHEN** no hay eventos de uso en el día actual
- **THEN** el popover muestra un estado vacío en la sección por proyecto

### Requirement: Refresco al abrir el popover
La aplicación SHALL consultar `query_limits` y `query_today_by_project` al mostrarse el popover, para reflejar datos frescos sin esperar al siguiente ciclo de poll, y SHALL además escuchar los eventos `limits-updated` y `usage-updated`.

#### Scenario: Refresco inmediato al abrir
- **WHEN** el usuario abre el popover
- **THEN** la app consulta límites y consumo por proyecto de inmediato y actualiza la vista

#### Scenario: Refresco por nuevos datos de uso
- **WHEN** el backend emite `usage-updated`
- **THEN** la sección de consumo por proyecto del día se vuelve a consultar y actualizar

