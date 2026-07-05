## Context

TokenWatch ya tiene una infraestructura de release y actualización madura:

- **Updater**: `tauri-plugin-updater` registrado en Rust (`src-tauri/src/lib.rs`), endpoint `releases.tokenwatch.gulloa.click/latest.json`, pubkey y permisos (`updater:default`, `process:allow-restart`) configurados.
- **Frontend de updates**: `features/updates/` con `useAppUpdate` (chequeo al arranque + cada 6h), `UpdateBanner` (montado en `app/Popover.tsx`) y `updaterClient` (guardas para entornos no-Tauri).
- **Changelog de fuente única**: `CHANGELOG.md` en la raíz (formato Keep a Changelog), sincronizado al frontend por `scripts/sync-changelog.mjs` → `src/generated/changelog.md` (gitignored, consumido vía import `?raw`). `bump-version.mjs::promoteUnreleased` promueve `[Unreleased]` → `[X.Y.Z] - <fecha>` en cada release.
- **Release**: `scripts/release.sh` (dev → release/vX.Y.Z → master → tag) + `.github/workflows/release.yml` (build multiplataforma, extrae notas del CHANGELOG con fallback a notas auto de GitHub, publica `latest.json` a S3/CloudFront).

Lo que falta y motiva este cambio: (1) no hay superficie estable para ver la versión actual ni acceder al changelog; (2) tras actualizar, el usuario no ve "qué cambió"; (3) el `## [Unreleased]` es manual y hoy está vacío, así que las notas salen genéricas. El repo ya usa mensajes de commit con convención (`feat(app):`, `fix(app):`, `chore(release):`), lo que habilita generar el changelog automáticamente.

## Goals / Non-Goals

**Goals:**
- Exponer la versión actual en runtime en una superficie "Acerca de" tipo Argus, con acción de actualizar y acceso al changelog.
- Mostrar una vez un modal *What's New* cuando la versión en ejecución cambia respecto a la última vista.
- Generar automáticamente la sección `[Unreleased]` del `CHANGELOG.md` desde los commits, integrándolo al flujo de release sin romper el modelo de fuente única.
- Reutilizar al máximo lo existente: `useAppUpdate`, el import `?raw` del changelog, `promoteUnreleased`, `sync-changelog.mjs`.

**Non-Goals:**
- Reescribir el flujo de actualización (`useAppUpdate`/`UpdateBanner`) — solo se consume.
- Cambios en Rust o en la configuración del updater.
- Cambios en la infraestructura AWS/CDK ni en el hosting del manifiesto.
- Un editor de changelog in-app o edición manual asistida.
- Internacionalización más allá del español ya usado en la UI.

## Decisions

### 1. Versión en runtime vía `@tauri-apps/api/app::getVersion()`

Se obtiene la versión con `getVersion()` de `@tauri-apps/api/app` (disponible en Tauri 2), no con un valor hardcodeado ni un comando Rust custom. Devuelve la versión de `tauri.conf.json` embebida en el binario — la fuente de verdad ya sincronizada por `bump-version.mjs`.

- **Guarda no-Tauri**: se envuelve en un cliente con `try/catch` y import dinámico, igual que el patrón `safeTauriInvoke`/`updaterClient` existente, devolviendo `null` fuera de Tauri para degradar a `v—`/"dev".
- **Alternativa descartada**: comando Rust `#[tauri::command] get_version` → innecesario, agrega superficie Rust y capabilities sin beneficio sobre la API JS.

### 2. Superficie "Acerca de": sección accesible, no ventana nueva

La superficie vive como una vista/sección alcanzable desde un punto de entrada en el popover (footer, junto a "Open dashboard") y/o en el dashboard `App.tsx`. Se implementa en `features/about/` (componente + `useAppVersion`). Reutiliza `useAppUpdate` para la acción de actualizar y el import `?raw` del changelog para "ver changelog completo".

- **Alternativa descartada**: ventana WebView independiente → mayor complejidad (config Tauri, ciclo de vida) sin necesidad; la app ya usa vistas dentro de las ventanas existentes.

### 3. *What's New*: comparación de versión + persistencia local

Al arranque, `useWhatsNew` compara `getVersion()` con la "última versión vista" persistida. Persistencia en `localStorage` bajo una clave versionada (p. ej. `tokenwatch.lastSeenVersion`).

- **Semántica de primera instalación**: si no hay valor previo, se registra la versión actual como vista y **no** se muestra el modal (evita ruido al instalar por primera vez).
- **Extracción de la sección**: un parser puro `extractVersionSection(changelogMarkdown, version)` localiza `## [X.Y.Z]` y devuelve su cuerpo hasta la siguiente `## `. Se comparte con "ver changelog completo".
- **Contenido**: se renderiza la sección de la versión actual (ya promovida en el binario publicado). Si no hay coincidencia, no se muestra el modal pero igual se marca como vista (evita reintentos por arranque).
- **Alternativa descartada**: usar `update.notes` del updater → esas notas solo existen durante el flujo de actualización y desaparecen tras instalar; el changelog empaquetado es persistente y funciona aunque la app se haya actualizado por fuera.
- **Alternativa descartada**: Tauri Store plugin → `localStorage` basta para un flag simple y evita una dependencia/capability nueva.

### 4. Generación automática del changelog: script `generate-changelog.mjs`

Nuevo `packages/app/scripts/generate-changelog.mjs` con una función pura testeable `commitsToChangelog(commits)` que:

1. Lee commits desde el último tag (`git log <lastTag>..HEAD --pretty=...`) — la I/O de git queda fuera de la función pura para poder testear commits→markdown.
2. Parsea Conventional Commits (`type(scope): subject`) y agrupa por categoría Keep a Changelog: `feat` → **Added**, `fix` → **Fixed**, `perf`/`refactor` → **Changed**; tipos como `chore`/`ci`/`docs`/`test`/`build` se ignoran para las notas del usuario (configurable).
3. Reescribe únicamente el cuerpo de `## [Unreleased]` en `CHANGELOG.md`, preservando el resto del archivo y el formato Keep a Changelog.

**Integración**: se invoca en el flujo de release **antes** de `bump-version.mjs`, de modo que `promoteUnreleased` promueva contenido real. Punto de integración preferente: un paso previo en `release.sh` (y/o en `release.yml`), sin tocar el contrato de `bump-version.mjs` ni `sync-changelog.mjs`. Como `release.yml` ya hace fallback a notas auto de GitHub cuando `[Unreleased]` está vacío, el comportamiento sigue siendo seguro si el generador no produce entradas.

- **Alternativa descartada**: dependencia externa tipo `conventional-changelog`/`standard-version` → agrega peso y opinión; el parser propio es pequeño, testeable y encaja con el formato ya adoptado.
- **Alternativa descartada**: generar notas solo en CI a partir de la API de GitHub → rompe el modelo de fuente única (`CHANGELOG.md`) que ya alimenta al frontend.

## Risks / Trade-offs

- **[Commits sin convención quedan fuera del changelog]** → El generador ignora lo que no matchea; se documenta la convención y `release.yml` mantiene el fallback a notas auto de GitHub para no publicar un release sin notas. Se puede añadir una categoría "Other" si se decide no perder nada.
- **[`localStorage` se limpia → el modal reaparece]** → Impacto bajo (a lo más se muestra una vez de más); aceptable frente a la simplicidad. No se usa para nada crítico.
- **[Desalineación entre la versión del binario y la sección del changelog]** → Si `generate-changelog` no corrió antes de `promoteUnreleased`, la sección de la versión puede faltar; el modal degrada a "sin novedades" y marca como vista. Se mitiga fijando el orden en el flujo de release y con un test del orden/`promoteUnreleased`.
- **[Parser de Markdown frágil ante formato inesperado]** → Se mantiene el parser mínimo (localizar `## [version]` hasta el siguiente `## `), con tests de casos borde (versión inexistente, secciones vacías, encabezado con fecha).

## Migration Plan

Cambio aditivo, sin migración de datos ni rollback especial:

1. Frontend (`features/about/`, `features/whats-new/`, parser compartido) — no altera flujos existentes; los puntos de entrada se agregan al popover/dashboard.
2. Script `generate-changelog.mjs` — se añade y se cablea en `release.sh`/`release.yml` antes de `bump-version.mjs`.
3. Rollback: revertir el cambio; al ser aditivo, no deja estado persistente relevante (más allá de una clave `localStorage` inocua).

## Open Questions

- **Punto de entrada de "Acerca de"**: ¿footer del popover, dashboard, o ambos? (Decisión de UI, no bloquea el diseño técnico.)
- **Tipos incluidos en el changelog**: confirmar el mapeo definitivo (¿se incluye `docs`/`chore`? ¿categoría "Other" para commits sin convención?).
- **Dónde cablear el generador**: ¿en `release.sh` (local, previo al PR de release) o como paso de `release.yml`? Preferencia: `release.sh` para que el `[Unreleased]` quede versionado en el PR de release y sea revisable.
