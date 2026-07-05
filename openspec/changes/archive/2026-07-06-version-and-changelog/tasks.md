## 1. Versión en runtime y parser de changelog (base compartida)

- [x] 1.1 Crear `features/about/appVersionClient.ts`: wrapper con import dinámico y `try/catch` sobre `getVersion()` de `@tauri-apps/api/app`, devolviendo `null` fuera de Tauri (patrón `updaterClient`).
- [x] 1.2 Crear hook `features/about/useAppVersion.ts` que exponga `{ version, isTauri }` y degrade a placeholder (`v—`/"dev") cuando no hay versión.
- [x] 1.3 Crear parser puro `features/whats-new/changelogParser.ts` con `extractVersionSection(markdown, version)` (localiza `## [X.Y.Z]` hasta el siguiente `## `) y `getLatestVersionSection(markdown)`.
- [x] 1.4 Tests Vitest de `changelogParser`: versión existente, inexistente, sección vacía, encabezado con fecha, changelog sin secciones.

## 2. Superficie "Acerca de / Versión"

- [x] 2.1 Crear componente `features/about/AboutSection.tsx`: muestra `vX.Y.Z` (de `useAppVersion`), acción "Buscar actualizaciones"/instalar (vía `useAppUpdate`) y estado, y enlace "Ver changelog".
- [x] 2.2 Renderizar el changelog completo desde el import `?raw` de `src/generated/changelog.md` (vista/modal legible).
- [x] 2.3 Agregar punto de entrada a "Acerca de" en `app/Popover.tsx` (footer, junto a "Open dashboard") y/o en `app/App.tsx`.
- [x] 2.4 Tests Vitest de `AboutSection`: muestra versión, refleja estados de update (`available`/`ready`/`idle`), abre changelog; caso no-Tauri con placeholder.

## 3. Modal "Novedades / What's New"

- [x] 3.1 Crear `features/whats-new/lastSeenVersion.ts`: get/set de `tokenwatch.lastSeenVersion` en `localStorage` con guardas.
- [x] 3.2 Crear hook `features/whats-new/useWhatsNew.ts`: compara `useAppVersion` con la última versión vista; maneja primera instalación (marca como vista, no muestra); expone `{ show, versionSection, dismiss }`.
- [x] 3.3 Crear componente `features/whats-new/WhatsNewModal.tsx` que renderiza la sección de la versión actual y al cerrar llama a `dismiss()`.
- [x] 3.4 Montar el modal en el arranque de la app (`app/App.tsx` o `main.tsx`) sin bloquear el resto de la UI.
- [x] 3.5 Tests Vitest de `useWhatsNew`: versión nueva → muestra; versión vista → no muestra; primera instalación → marca sin mostrar; sin sección → no muestra pero marca; cierre persiste.

## 4. Generación automática del changelog

- [x] 4.1 Crear `packages/app/scripts/generate-changelog.mjs` con función pura exportada `commitsToChangelog(commits)` (Conventional Commits → categorías Keep a Changelog: feat→Added, fix→Fixed, perf/refactor→Changed; chore/ci/docs/test/build ignorados).
- [x] 4.2 Implementar la lectura de commits desde el último tag (`git log <lastTag>..HEAD`) y la reescritura del cuerpo de `## [Unreleased]` en `CHANGELOG.md`, preservando el resto y el formato Keep a Changelog.
- [x] 4.3 Cablear la ejecución del generador en el flujo de release ANTES de `bump-version.mjs` (`scripts/release.sh` y/o `.github/workflows/release.yml`).
- [x] 4.4 Tests Vitest de `commitsToChangelog`: agrupación por tipo, commits sin convención omitidos, sin cambios → sección vacía, orden estable.
- [x] 4.5 Verificar que `promoteUnreleased` y `sync-changelog.mjs` siguen funcionando con el contenido generado (test o verificación manual del orden en el flujo).

## 5. Validación y cierre

- [x] 5.1 Correr `pnpm typecheck && pnpm lint && pnpm test:run` en `packages/app` y dejarlos en verde.
- [x] 5.2 Verificación manual: abrir "Acerca de" (versión visible + acción de update + changelog) y simular cambio de versión para ver el modal *What's New* una sola vez.
- [x] 5.3 Actualizar documentación relevante (README/CLAUDE.md o docs de release) sobre la generación automática del changelog y la convención de commits.
