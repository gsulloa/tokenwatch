# changelog-markdown-rendering Specification

## Purpose

Proveer un componente reutilizable que renderice contenido markdown estilo Keep-a-Changelog como elementos visuales legibles, alineado al sistema de diseño, seguro frente a HTML crudo, y consumible desde cualquier superficie de changelog de la app.

## Requirements

### Requirement: Renderizado de markdown de changelog

La app SHALL renderizar el contenido de changelog (markdown estilo Keep-a-Changelog) como elementos visuales legibles, no como texto plano. El renderizador SHALL soportar como mínimo: encabezados (`##`, `###`), listas con viñetas incluyendo anidación, énfasis (`**negrita**`, `*cursiva*`), código en línea (`` `code` ``), enlaces `[texto](url)` y párrafos separados por líneas en blanco.

#### Scenario: Encabezados se renderizan como títulos

- **WHEN** el contenido incluye una línea `### Added`
- **THEN** se renderiza como un encabezado visual (elemento de título), no como el texto literal `### Added`

#### Scenario: Listas se renderizan como viñetas

- **WHEN** el contenido incluye líneas que empiezan con `- `
- **THEN** se renderizan como una lista con viñetas, y las líneas con mayor indentación se renderizan como sublistas anidadas

#### Scenario: Énfasis y código en línea se renderizan

- **WHEN** el contenido incluye `**texto**`, `*texto*` o `` `texto` ``
- **THEN** se renderizan respectivamente como negrita, cursiva y código en línea, sin mostrar los marcadores markdown

#### Scenario: Enlaces se renderizan y abren de forma segura

- **WHEN** el contenido incluye un enlace `[texto](https://ejemplo.com)`
- **THEN** se renderiza como un enlace clickeable con el texto visible
- **AND** al activarlo el destino se abre en el navegador externo (no navega dentro de la ventana de la app)

### Requirement: Renderizado alineado al sistema de diseño

El renderizador de markdown SHALL usar los tokens de diseño definidos en `DESIGN.md` (colores, tipografía, espaciado, radios) para todos los elementos que produce, sin introducir estilos ajenos al sistema.

#### Scenario: Elementos usan tokens de diseño

- **WHEN** el renderizador produce encabezados, listas, párrafos o enlaces
- **THEN** sus estilos derivan de las variables/tokens del sistema de diseño (p. ej. `var(--text)`, `var(--accent)`, `var(--space-*)`), consistentes con el resto de la UI

### Requirement: Seguridad frente a HTML crudo

El renderizador SHALL tratar el contenido como markdown de confianza limitada: NO SHALL interpretar ni inyectar HTML crudo embebido en el texto, para evitar inyección de marcado o scripts.

#### Scenario: HTML embebido no se ejecuta

- **WHEN** el contenido de changelog incluye una cadena tipo `<img src=x onerror=alert(1)>` o `<script>...</script>`
- **THEN** el renderizador NO ejecuta ni inserta ese HTML como marcado activo (se ignora o se muestra como texto), sin ejecutar scripts

### Requirement: Reutilización en superficies de changelog

El renderizador de markdown SHALL ser un componente reutilizable, consumido tanto por la superficie de "Novedades / What's New" como por la de "Changelog completo", de modo que ambas presenten el mismo formato.

#### Scenario: Mismo renderizador en ambas superficies

- **WHEN** se muestran las notas de una versión en el modal de Novedades y el changelog completo en el dashboard
- **THEN** ambas usan el mismo componente de render de markdown y presentan el mismo formato visual para las mismas construcciones markdown
