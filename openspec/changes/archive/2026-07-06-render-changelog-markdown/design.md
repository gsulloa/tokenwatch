## Context

El contenido de changelog ya llega a la UI correctamente (extraído por `changelogParser.ts` para Novedades, y el `changelog.md` empaquetado completo para el modal de Changelog). El problema es puramente de presentación:

- `packages/app/src/features/about/ChangelogModal.tsx` pinta el texto dentro de un `<pre>` con `white-space: pre-wrap`.
- `packages/app/src/features/whats-new/WhatsNewModal.tsx` pinta el texto dentro de un `<div>` con `white-space: pre-wrap`.

En ambos casos el markdown (`##`, `-`, `**bold**`, `[link](url)`) se muestra literal. El código actual documenta explícitamente que evitó dependencias de markdown ("no external markdown dependencies needed"), y el proyecto ya tiene un patrón de parsers puros sin dependencias y con tests unitarios (`changelogParser.ts`).

Restricciones:
- Debe respetar `DESIGN.md` (tokens de color/tipografía/espaciado).
- App Tauri: los enlaces externos deben abrirse en el navegador del sistema, no dentro de la ventana.
- Seguridad: el contenido viene de un `CHANGELOG.md` empaquetado (confianza razonable) pero NO debe abrirse la puerta a inyección de HTML crudo.

## Goals / Non-Goals

**Goals:**
- Renderizar el subconjunto de markdown Keep-a-Changelog como elementos visuales: encabezados (`##`/`###`), listas con viñetas y anidadas, `**bold**`, `*italic*`, `` `code` ``, enlaces `[texto](url)`, párrafos.
- Un único componente reutilizado por Novedades y Changelog completo.
- Estilos derivados de los tokens de `DESIGN.md`.
- Enlaces externos que abren de forma segura en el navegador del sistema.
- Cobertura de tests unitarios del parser/render, en línea con el estilo de `changelogParser.test.ts`.

**Non-Goals:**
- Soporte completo de CommonMark/GFM (tablas, blockquotes, imágenes, HTML embebido, footnotes). Solo el subconjunto que aparece en un changelog.
- Cambiar el pipeline de generación de changelog (`auto-changelog-generation`) ni el formato del `CHANGELOG.md`.
- Cambiar la lógica de detección de versión o extracción de secciones (`useWhatsNew`, `changelogParser`).

## Decisions

### Decisión 1: Renderizador propio a React elements (sin dependencia externa)

Se implementa un componente `Markdown` (p. ej. `packages/app/src/components/Markdown/`) que parsea el subconjunto necesario y emite **elementos React** directamente (`<h3>`, `<ul>`, `<li>`, `<strong>`, `<em>`, `<code>`, `<a>`, `<p>`).

**Por qué sobre las alternativas:**
- **`react-markdown` (+ remark/rehype):** robusto y seguro por defecto, pero añade varias dependencias y peso de bundle a una app de menu-bar liviana; es sobredimensionado para un subconjunto pequeño. Además requeriría configurar plugins para estilos con tokens y para el manejo de enlaces en Tauri.
- **`marked` + `dangerouslySetInnerHTML`:** ligero pero obliga a un sanitizador (DOMPurify) para ser seguro; introduce superficie de inyección justo lo que queremos evitar.
- **Parser propio:** encaja con el patrón ya establecido en el repo (parsers puros, testeables, sin deps), es inherentemente seguro frente a HTML crudo (solo emitimos elementos React conocidos; cualquier `<...>` del input se trata como texto), y da control total sobre estilos con tokens y sobre el handler de enlaces. El subconjunto es acotado y estable.

Riesgo aceptado: mantener un mini-parser. Se mitiga con tests y con un alcance deliberadamente pequeño.

### Decisión 2: Seguridad por construcción

Al emitir solo elementos React (nunca `dangerouslySetInnerHTML`), cualquier HTML crudo en el input (`<script>`, `<img onerror=...>`) se renderiza como texto plano escapado por React, no como marcado activo. No se necesita sanitizador adicional.

### Decisión 3: Manejo de enlaces en Tauri

Los `<a>` generados usan un handler que abre el destino en el navegador del sistema. Preferencia: usar el plugin/API de apertura externa de Tauri (`@tauri-apps/plugin-shell` `open`, o `openUrl` según lo disponible) con fallback a `target="_blank"` + `rel="noopener noreferrer"` cuando se corre fuera de Tauri (dev en browser). Solo se aceptan esquemas `http`/`https`/`mailto`; otros esquemas se renderizan como texto para evitar `javascript:`.

### Decisión 4: Estrategia de parseo

Parseo por líneas orientado a bloques (headings, items de lista con indentación, párrafos), y un pase inline sobre el texto de cada bloque para `**`, `*`, `` ` `` y `[]()`. Se reutiliza el enfoque de funciones puras: una función `parse(markdown) -> nodes` y un componente que mapea nodes a React, para poder testear el parseo por separado del render.

### Decisión 5: Integración en las superficies

- `ChangelogModal`: reemplazar el `<pre>{changelogText}</pre>` por `<Markdown source={changelogText} />`.
- `WhatsNewModalView`: reemplazar el `<div>{versionSection}</div>` por `<Markdown source={versionSection} />`, conservando el fallback "Sin notas para esta versión." cuando no hay contenido.

## Risks / Trade-offs

- [El mini-parser no cubre alguna construcción markdown que aparezca en el futuro] → Alcance documentado; construcciones no soportadas caen a texto plano legible (degradación suave), y el `CHANGELOG.md` es generado con formato controlado.
- [Diferencias de comportamiento de apertura de enlaces entre Tauri y browser dev] → Fallback explícito a `target="_blank"` + `rel="noopener noreferrer"` fuera de Tauri; tests cubren la construcción del `<a>`.
- [Regresión visual vs. el layout actual] → Estilos anclados a tokens de `DESIGN.md`; QA visual de ambos modales.

## Open Questions

- ¿Usar `@tauri-apps/plugin-shell` (`open`) o el helper de `opener`/`openUrl` disponible en la versión de Tauri del proyecto? Verificar qué está instalado/permitido en `tauri.conf` antes de implementar. Si añadir el plugin es fricción, usar `window.open` como handler único es aceptable dado que es contenido de confianza.
