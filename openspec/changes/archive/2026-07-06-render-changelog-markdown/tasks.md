## 1. Parser de markdown

- [x] 1.1 Crear `packages/app/src/components/Markdown/parseMarkdown.ts` con una función pura `parseMarkdown(source: string)` que produzca nodos de bloque (heading, ul/li con anidación, párrafo) e inline (bold, italic, code, link, text)
- [x] 1.2 Implementar el pase inline para `**bold**`, `*italic*`, `` `code` `` y `[texto](url)`, tratando cualquier `<...>` del input como texto (sin HTML crudo)
- [x] 1.3 Validar esquemas de enlace: aceptar solo `http`/`https`/`mailto`; otros se degradan a texto
- [x] 1.4 Escribir `parseMarkdown.test.ts` cubriendo headings, listas anidadas, énfasis, código, enlaces, esquema inválido y HTML crudo tratado como texto

## 2. Componente de render

- [x] 2.1 Crear `packages/app/src/components/Markdown/Markdown.tsx` que mapee los nodos a elementos React (`h2`/`h3`, `ul`/`li`, `strong`, `em`, `code`, `a`, `p`), usando tokens de `DESIGN.md` para todos los estilos
- [x] 2.2 Implementar el handler de enlaces: abrir en navegador del sistema vía API de Tauri disponible, con fallback a `target="_blank"` + `rel="noopener noreferrer"` fuera de Tauri (resolver Open Question del design leyendo `tauri.conf`/deps)
- [x] 2.3 Escribir `Markdown.test.tsx` verificando que no se usa `dangerouslySetInnerHTML`, que el HTML crudo no se ejecuta, y que los enlaces se construyen con `rel`/handler correctos

## 3. Integración en superficies

- [x] 3.1 En `packages/app/src/features/about/ChangelogModal.tsx`, reemplazar el `<pre>{changelogText}</pre>` por `<Markdown source={changelogText} />`
- [x] 3.2 En `packages/app/src/features/whats-new/WhatsNewModal.tsx`, reemplazar el `<div>{versionSection}</div>` por `<Markdown source={versionSection} />`, conservando el fallback "Sin notas para esta versión." cuando no hay contenido
- [x] 3.3 Actualizar/añadir tests de `AboutSection`/`WhatsNew` si asumían texto plano

## 4. Verificación

- [ ] 4.1 Verificar visualmente (dev) ambos modales: headings, listas anidadas, negrita, código y enlaces se ven renderizados y alineados a `DESIGN.md` (pendiente: QA visual en app corriendo)
- [x] 4.2 Ejecutar `pnpm typecheck && pnpm lint && pnpm test:run` y dejarlos en verde
