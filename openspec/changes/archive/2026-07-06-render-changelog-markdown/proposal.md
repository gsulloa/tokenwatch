## Why

El changelog se muestra correctamente en la UI (modal de Novedades y modal de Changelog completo), pero su contenido se pinta como texto plano: los encabezados `###`, las viñetas `-`, el **negrita** y los enlaces aparecen como markdown literal en lugar de renderizarse. Esto contradice los specs existentes, que piden que las notas se muestren "renderizadas de forma legible", y da una impresión pobre justo en la superficie que comunica el valor de cada release.

## What Changes

- Introducir un renderizador de markdown para las notas de changelog, reemplazando los bloques de texto plano (`<pre>` en `ChangelogModal` y `<div white-space:pre-wrap>` en `WhatsNewModal`).
- Soportar el subconjunto de markdown que realmente usa un `CHANGELOG.md` estilo Keep-a-Changelog: encabezados (`##`/`###`), listas con viñetas y anidadas, énfasis (`**bold**`, `*italic*`, `` `code` ``), enlaces `[texto](url)` y párrafos.
- El componente de render debe respetar el sistema de diseño (tokens de `DESIGN.md`: colores, tipografía, espaciado) y no introducir estilos ajenos.
- Los enlaces deben abrirse de forma segura (nueva ventana / handler externo apropiado para una app Tauri), sin permitir HTML crudo embebido en el markdown (evitar inyección).
- Reutilizar el mismo renderizador en ambas superficies (Novedades y Changelog completo) para consistencia.

## Capabilities

### New Capabilities
- `changelog-markdown-rendering`: renderizado del contenido de changelog (markdown Keep-a-Changelog) como elementos visuales legibles —encabezados, listas, énfasis y enlaces— reutilizable por las superficies de Novedades y Changelog completo, alineado al sistema de diseño y seguro frente a HTML crudo.

### Modified Capabilities
- `whats-new-on-update`: el escenario "Modal con las notas de la versión" se precisa: el contenido de la versión SHALL renderizarse como markdown (encabezados, listas, énfasis, enlaces), no como texto plano.
- `app-version-display`: los escenarios de "Ver changelog completo" se precisan: el changelog empaquetado SHALL renderizarse como markdown, no como texto plano.

## Impact

- `packages/app/src/features/about/ChangelogModal.tsx` — reemplaza el `<pre>` por el renderizador de markdown.
- `packages/app/src/features/whats-new/WhatsNewModal.tsx` — reemplaza el `<div>` de texto plano por el renderizador.
- Nuevo módulo de render de markdown en `packages/app/src/` (componente + tests).
- Posible nueva dependencia ligera de markdown (p. ej. `marked` o `react-markdown`) o un mini-parser propio; se decide en `design.md`. Afecta `packages/app/package.json` si se añade dependencia.
- Sin cambios en backend, ni en el pipeline de generación de changelog (`auto-changelog-generation`).
