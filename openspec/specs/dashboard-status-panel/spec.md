# dashboard-status-panel Specification

## Purpose
TBD - created by archiving change dashboard-usage-limits-panel. Update Purpose after archive.
## Requirements
### Requirement: Panel de estado en vivo en el dashboard

La ventana del **dashboard** SHALL mostrar, además de su análisis histórico, un
panel de estado en vivo con las mismas lecturas que ofrece el popover: los
**medidores de límites** (sesión 5h, semana total y semanales por modelo cuando
existan), los **presupuestos por grupo** (cuando haya grupos definidos) y el
**consumo de hoy por proyecto**. El contenido y las reglas de presentación de
cada sección (porcentajes, referencias de reset, medidores con ticks, mensajes
de "no disponible", orden de proyectos, total del día) SHALL ser idénticos a los
del popover, reutilizando los mismos componentes de sección.

#### Scenario: Dashboard con datos de límites

- **WHEN** el usuario abre el dashboard y hay un snapshot de límites disponible
- **THEN** el dashboard muestra los medidores de sesión 5h y semana con su
  porcentaje y referencia de reset, y una sub-sección por modelo cuando el
  snapshot incluya semanales por modelo

#### Scenario: Dashboard con presupuestos por grupo

- **WHEN** el usuario tiene al menos un grupo de proyectos definido
- **THEN** el dashboard muestra la sección de presupuestos por grupo con sus
  medidores, igual que el popover

#### Scenario: Dashboard sin grupos definidos

- **WHEN** el usuario no tiene ningún grupo definido
- **THEN** el dashboard omite la sección de presupuestos por grupo, igual que el
  popover

#### Scenario: Dashboard con consumo de hoy

- **WHEN** hoy hubo consumo por proyecto
- **THEN** el dashboard lista cada proyecto con sus tokens y su porcentaje del
  total del día, ordenados de mayor a menor, más el total del día

#### Scenario: Límites no disponibles en el dashboard

- **WHEN** los límites están no disponibles (p. ej. token expirado o no logueado)
- **THEN** el dashboard muestra el mismo mensaje explicativo que el popover en
  lugar de porcentajes en blanco

### Requirement: Paridad de contenido entre popover y dashboard

El sistema SHALL derivar las lecturas en vivo (límites, presupuestos por grupo y
consumo de hoy por proyecto) de una **única fuente de composición compartida**,
de modo que ambas superficies muestren siempre las mismas secciones en el mismo
orden. Agregar, quitar o reordenar una de estas secciones MUST reflejarse
simultáneamente en el popover y en el dashboard sin duplicar la lógica de
composición.

#### Scenario: Misma información en ambos lados

- **WHEN** el mismo snapshot de límites, presupuestos y consumo de hoy está
  disponible
- **THEN** el popover y el dashboard muestran las mismas secciones con los mismos
  valores

#### Scenario: Cambio de composición se propaga a ambos

- **WHEN** se modifica el conjunto o el orden de las secciones en vivo
- **THEN** el cambio aparece tanto en el popover como en el dashboard, sin
  requerir editar dos composiciones separadas

### Requirement: Sincronización en vivo del panel del dashboard

El panel de estado en vivo del dashboard SHALL mantenerse actualizado ante los
mismos eventos del backend que el popover: al recibir `limits-updated` los
medidores de límites (y presupuestos por grupo) SHALL refrescarse, y al recibir
`usage-updated` el consumo de hoy (y presupuestos) SHALL refrescarse. Al abrir el
dashboard, el panel SHALL consultar el estado actual. El gesto de "Actualizar"
del dashboard SHALL forzar además un refresco de las lecturas en vivo.

#### Scenario: Actualización por evento de límites

- **WHEN** el backend emite `limits-updated` mientras el dashboard está abierto
- **THEN** los medidores de límites del dashboard se actualizan con el nuevo
  snapshot

#### Scenario: Actualización por evento de uso

- **WHEN** el backend emite `usage-updated` mientras el dashboard está abierto
- **THEN** el consumo de hoy por proyecto del dashboard se actualiza

#### Scenario: Refresco manual desde el dashboard

- **WHEN** el usuario pulsa "Actualizar" en el dashboard
- **THEN** además de la serie histórica, las lecturas en vivo (límites y consumo
  de hoy) se vuelven a consultar

