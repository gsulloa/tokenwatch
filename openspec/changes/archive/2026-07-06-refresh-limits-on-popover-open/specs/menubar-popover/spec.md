## ADDED Requirements

### Requirement: Actualización de límites al abrir el popover
Cada vez que el popover se muestre, el frontend SHALL solicitar un snapshot de límites recién obtenido (vía el comando `query_limits`, que consulta la API de uso en vivo), de modo que los medidores de sesión y semana reflejen datos actuales al abrir. Esta actualización al abrir SHALL ser adicional e independiente del ciclo de polling en segundo plano, el cual MUST permanecer sin cambios como mecanismo que dispara las notificaciones de umbral. Dado que el webview del popover persiste entre aperturas, la actualización SHALL dispararse a partir del evento `popover-shown` (no solo al montar el componente).

Mientras la nueva consulta está en curso, el popover MUST seguir mostrando el último snapshot conocido (no debe quedar en blanco ni parpadear), y SHALL reemplazar los valores en su lugar cuando la consulta fresca se resuelva con éxito. Para evitar llamadas redundantes ante aperturas/cierres rápidos, la actualización al abrir SHALL respetar un intervalo mínimo entre consultas: si una consulta exitosa se completó dentro de esa ventana reciente, la apertura MUST omitir la nueva consulta y conservar los valores mostrados.

#### Scenario: Datos frescos al abrir
- **WHEN** el usuario abre el popover y ha pasado más que el intervalo mínimo desde la última consulta
- **THEN** el frontend consulta `query_limits` y actualiza los medidores con el snapshot recién obtenido

#### Scenario: Sin parpadeo mientras carga
- **WHEN** el usuario abre el popover con un snapshot previo ya mostrado y la nueva consulta está en curso
- **THEN** los medidores siguen mostrando los valores previos hasta que la consulta fresca se resuelve, sin quedar en blanco

#### Scenario: Aperturas rápidas no duplican consultas
- **WHEN** el usuario abre y cierra el popover repetidamente dentro del intervalo mínimo entre consultas
- **THEN** solo se realiza una consulta y las aperturas siguientes reutilizan los valores ya mostrados

#### Scenario: El polling sigue impulsando notificaciones
- **WHEN** transcurre el intervalo de polling en segundo plano
- **THEN** el ciclo de polling continúa obteniendo el snapshot, evaluando umbrales y disparando notificaciones, independientemente de si el popover se abrió o no
