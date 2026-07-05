## Why

Hoy el usuario no tiene forma de saber en qué versión de TokenWatch está: la versión solo se vuelve visible de refilón cuando el `UpdateBanner` detecta una actualización, y las notas de cambios (`update.notes`) desaparecen apenas se instala. No hay una superficie estable tipo "Acerca de / Versión" (como la que ofrece Argus) para consultar la versión actual, forzar un chequeo y —sobre todo— entender *qué cambió* después de actualizar. Además, el `## [Unreleased]` de `CHANGELOG.md` se mantiene a mano y hoy está vacío, así que las notas de release y del updater salen genéricas (fallback de GitHub) en vez de reflejar los commits reales.

## What Changes

- **Versión visible en la UI**: nueva sección/superficie "Acerca de" que muestra la versión actual (`vX.Y.Z`) obtenida en runtime vía `getVersion()` de `@tauri-apps/api/app`, con acción explícita para buscar/instalar actualizaciones (reutilizando el hook `useAppUpdate` existente) y un enlace para ver el changelog completo.
- **"Novedades" al cambiar de versión**: al arrancar, si la versión en ejecución difiere de la última versión "vista" (persistida), se muestra una vez un modal *What's New* que renderiza la sección del changelog correspondiente a la nueva versión desde el `CHANGELOG.md` empaquetado (`src/generated/changelog.md`). Al cerrarlo se marca la versión como vista y no vuelve a aparecer.
- **Changelog generado automáticamente**: nuevo script que, a partir del historial de commits con convención (`feat`, `fix`, `perf`, etc.) desde el último tag, rellena la sección `## [Unreleased]` de `CHANGELOG.md` agrupada por tipo (Added / Fixed / Changed…). Se integra en el flujo de release **antes** de que `bump-version.mjs` promueva `[Unreleased]` a la versión fechada, de modo que las notas del release de GitHub, del manifiesto del updater y del modal *What's New* queden alimentadas por los mismos datos, sin edición manual.

## Capabilities

### New Capabilities
- `app-version-display`: superficie "Acerca de / Versión" en la app — muestra la versión actual en runtime, expone la acción de buscar/instalar actualización y da acceso al changelog completo.
- `whats-new-on-update`: detección de cambio de versión y presentación única del changelog de la nueva versión (modal *What's New*), con persistencia de "última versión vista".
- `auto-changelog-generation`: generación automática de la sección `[Unreleased]` del `CHANGELOG.md` a partir de commits con convención, integrada al flujo de release existente.

### Modified Capabilities
<!-- Ninguna capability existente cambia sus requerimientos. `app-update-check` (del cambio frontend-update-check, aún sin archivar en openspec/specs) sigue igual; estas capabilities lo consumen sin modificar su comportamiento. `menubar-popover` puede alojar un punto de entrada a "Acerca de" pero su contrato de comportamiento no cambia. -->

## Impact

- **Frontend (`packages/app/src`)**:
  - Nuevo `features/about/` (o `features/version/`): componente de sección "Acerca de" + hook para versión en runtime.
  - Nuevo `features/whats-new/`: modal + hook de detección de cambio de versión (persistencia en `localStorage` o Tauri store) + parser que extrae la sección de una versión desde el changelog empaquetado.
  - Punto de entrada en `app/Popover.tsx` y/o `app/App.tsx` (dashboard) para abrir "Acerca de".
  - Reutiliza `features/updates/useAppUpdate` y el import `?raw` de `src/generated/changelog.md` ya existente.
- **Dependencias**: `@tauri-apps/api/app` (`getVersion`) — ya disponible con Tauri 2; sin nuevas dependencias de terceros para el parser (se implementa a mano sobre el formato Keep a Changelog).
- **Scripts / release (`packages/app/scripts`, `.github/workflows/release.yml`)**: nuevo `generate-changelog.mjs`; se invoca en el flujo de release (`release.sh` / workflow) antes de `bump-version.mjs`. No cambia el contrato de `sync-changelog.mjs` ni el de promoción de `[Unreleased]`.
- **Rust / config**: sin cambios (la versión en runtime se obtiene por API JS; el updater ya está configurado).
- **Tests**: unitarios del parser de changelog y del hook de detección de cambio de versión; del componente "Acerca de" y del modal *What's New*; del generador de changelog (función pura commits→markdown), siguiendo el patrón Vitest existente.
