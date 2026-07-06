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
Cada vez que el popover se muestre, el frontend SHALL solicitar datos recién obtenidos —un snapshot de límites (vía `query_limits`, que consulta la API de uso en vivo) y el consumo por proyecto del día (vía `query_today_by_project`)— de modo que los medidores de sesión/semana y el desglose por proyecto reflejen datos actuales al abrir, sin esperar al siguiente ciclo de poll. Dado que el webview del popover persiste entre aperturas, este refresco SHALL dispararse a partir del evento `popover-shown` (no solo al montar el componente). El refresco al abrir SHALL ser adicional e independiente del ciclo de polling en segundo plano, el cual MUST permanecer sin cambios como mecanismo que dispara las notificaciones de umbral. La app SHALL además escuchar los eventos `limits-updated` y `usage-updated` para actualizarse ante datos empujados por el backend.

Para los límites, mientras la nueva consulta está en curso, el popover MUST seguir mostrando el último snapshot conocido (no debe quedar en blanco ni parpadear), y SHALL reemplazar los valores en su lugar cuando la consulta fresca se resuelva con éxito. Para evitar llamadas redundantes ante aperturas/cierres rápidos, el refresco de límites al abrir SHALL respetar un intervalo mínimo entre consultas: si una consulta exitosa se completó dentro de esa ventana reciente, la apertura MUST omitir la nueva consulta de límites y conservar los valores mostrados.

#### Scenario: Datos frescos al abrir
- **WHEN** el usuario abre el popover y ha pasado más que el intervalo mínimo desde la última consulta de límites
- **THEN** el frontend consulta `query_limits` y `query_today_by_project` y actualiza los medidores y el consumo por proyecto con los datos recién obtenidos

#### Scenario: Sin parpadeo mientras carga
- **WHEN** el usuario abre el popover con un snapshot de límites previo ya mostrado y la nueva consulta está en curso
- **THEN** los medidores siguen mostrando los valores previos hasta que la consulta fresca se resuelve, sin quedar en blanco

#### Scenario: Aperturas rápidas no duplican consultas de límites
- **WHEN** el usuario abre y cierra el popover repetidamente dentro del intervalo mínimo entre consultas
- **THEN** solo se realiza una consulta de límites y las aperturas siguientes reutilizan los valores ya mostrados

#### Scenario: Refresco por nuevos datos de uso
- **WHEN** el backend emite `usage-updated`
- **THEN** la sección de consumo por proyecto del día se vuelve a consultar y actualizar

#### Scenario: El polling sigue impulsando notificaciones
- **WHEN** transcurre el intervalo de polling en segundo plano
- **THEN** el ciclo de polling continúa obteniendo el snapshot, evaluando umbrales y disparando notificaciones, independientemente de si el popover se abrió o no

