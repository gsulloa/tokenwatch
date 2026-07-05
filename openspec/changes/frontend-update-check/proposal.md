## Why

El plugin `tauri-plugin-updater` ya está registrado en Rust y configurado en `tauri.conf.json` (endpoint `releases.tokenwatch.gulloa.click/latest.json`, permisos `updater:default` + `process:allow-restart`), pero **el frontend nunca consulta si hay una versión nueva**. Sin esa pieza, la infraestructura de release está completa pero los usuarios jamás reciben actualizaciones: se quedan pegados en la versión que instalaron.

## What Changes

- Nuevo hook `useAppUpdate` que envuelve `check()` de `@tauri-apps/plugin-updater`: consulta el endpoint al arrancar la app y expone estado (`idle | checking | available | downloading | ready | error`), versión disponible y notas.
- Chequeo automático al iniciar (una vez) y re-chequeo periódico ligero, con guarda para entornos no-Tauri (dev en browser) igual que el patrón `safeTauriInvoke` existente.
- UI de notificación en el popover del menú-bar: banner/fila "Actualización disponible → vX.Y.Z" con acción para descargar e instalar.
- Flujo de instalación: `update.downloadAndInstall()` con progreso, y `relaunch()` (de `@tauri-apps/plugin-process`) al terminar para aplicar la actualización.
- Manejo de errores no intrusivo: si el chequeo falla (offline, endpoint caído), no se muestra ruido; se registra y se reintenta en el próximo ciclo.
- Acción manual "Buscar actualizaciones" para forzar el chequeo desde la UI.

## Capabilities

### New Capabilities
- `app-update-check`: chequeo de actualizaciones en el cliente — detección de versión nueva vía el endpoint del updater, notificación al usuario, descarga+instalación y relanzamiento de la app.

### Modified Capabilities
<!-- Ninguna: no cambian requerimientos de capabilities existentes (menubar-popover añade UI pero el comportamiento del popover como capability no cambia a nivel de requerimiento). -->

## Impact

- **Frontend (`packages/app/src`)**: nuevo `src/features/updates/` (hook + componente de UI); integración en `src/app/Popover.tsx`.
- **Dependencias**: usa `@tauri-apps/plugin-updater` y `@tauri-apps/plugin-process` (ya presentes en `package.json`); no requiere nuevas deps.
- **Rust / config**: sin cambios de código; el plugin y permisos ya existen. Persiste la dependencia externa de que `tauri.conf.json` tenga la `pubkey` real (hoy placeholder) para que la verificación de firma funcione en producción — fuera del alcance de este cambio de frontend.
- **Tests**: nuevos tests unitarios del hook (mock del plugin) y del componente de UI, siguiendo el patrón de Vitest ya usado en `features/usage`.
