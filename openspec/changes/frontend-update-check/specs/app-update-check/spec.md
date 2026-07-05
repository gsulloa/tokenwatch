## ADDED Requirements

### Requirement: Chequeo automático de actualizaciones al arrancar

La app SHALL consultar el endpoint del updater al iniciar para detectar si existe una versión más reciente que la instalada, sin bloquear el arranque ni la interacción del usuario.

#### Scenario: Existe una versión nueva

- **WHEN** la app arranca y el endpoint del updater reporta una versión mayor que la actual
- **THEN** el estado del updater pasa a `available` exponiendo la versión objetivo y sus notas de release

#### Scenario: La app está al día

- **WHEN** la app arranca y el endpoint no reporta una versión nueva
- **THEN** el estado del updater queda en `idle` y no se muestra ninguna notificación

#### Scenario: Entorno no-Tauri (dev en navegador)

- **WHEN** el código de chequeo se ejecuta fuera de un contexto Tauri (p.ej. Vitest o Vite en browser)
- **THEN** el chequeo se omite silenciosamente y el estado permanece en `idle` sin lanzar errores

### Requirement: Notificación de actualización disponible

La app SHALL notificar al usuario en el popover del menú-bar cuando haya una actualización disponible, incluyendo la versión objetivo y una acción para instalarla.

#### Scenario: Se muestra el aviso

- **WHEN** el estado del updater es `available`
- **THEN** el popover muestra una fila/banner "Actualización disponible → vX.Y.Z" con un botón para descargar e instalar

#### Scenario: No hay actualización

- **WHEN** el estado del updater es `idle`
- **THEN** el popover no muestra ningún elemento relacionado con actualizaciones

### Requirement: Descarga, instalación y relanzamiento

La app SHALL permitir al usuario descargar e instalar la actualización disponible y, al completarse, relanzar la app para aplicarla.

#### Scenario: Instalación exitosa

- **WHEN** el usuario acciona el botón de instalar sobre una actualización disponible
- **THEN** la app descarga e instala la actualización (mostrando estado `downloading`), pasa a `ready` al terminar y ofrece relanzar la app para aplicarla

#### Scenario: Progreso visible durante la descarga

- **WHEN** la descarga está en curso
- **THEN** la UI refleja el estado `downloading` y deshabilita reintentos duplicados del mismo botón

### Requirement: Chequeo manual de actualizaciones

La app SHALL exponer una acción para que el usuario fuerce un chequeo de actualizaciones bajo demanda.

#### Scenario: El usuario busca actualizaciones manualmente

- **WHEN** el usuario acciona "Buscar actualizaciones"
- **THEN** la app ejecuta el chequeo (estado `checking`) y actualiza el estado a `available` o `idle` según el resultado

### Requirement: Manejo de errores no intrusivo

La app SHALL manejar los fallos de chequeo o descarga sin interrumpir el uso normal ni mostrar ruido innecesario al usuario.

#### Scenario: Falla el chequeo por red u endpoint

- **WHEN** el chequeo o la descarga fallan (offline, endpoint caído, firma inválida)
- **THEN** el estado pasa a `error`, el error se registra, no se bloquea la app y un chequeo automático posterior puede reintentar

#### Scenario: El error no aparece si fue un chequeo automático de fondo

- **WHEN** un chequeo automático de fondo falla y el usuario no lo inició explícitamente
- **THEN** no se muestra un mensaje de error intrusivo en la UI
