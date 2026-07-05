# Design System — TokenWatch

> **Memorable thing:** *"Sé de un vistazo si estoy a punto de quedar rate-limited,
> antes de que pase."* TokenWatch no muestra datos — muestra **presión**: cuánto
> combustible queda y a qué velocidad se quema. Toda decisión de diseño sirve a esto.

## Product Context
- **What this is:** App de macOS en la barra de menú que monitorea el consumo de
  tokens de Claude / Codex por proyecto y workspace, con medidores de límite
  (5h / semanal), presupuestos por grupo y alertas.
- **Who it's for:** Desarrolladores que usan agentes de codificación AC intensivamente
  y necesitan no quedar rate-limited a mitad de una tarea.
- **Space/industry:** Herramientas de observabilidad / dev tooling. Peers de lenguaje
  visual: Linear, Raycast, Vercel, Datadog/Grafana, btop, Activity Monitor.
- **Project type:** Utilidad de escritorio con dos superficies —
  (1) **popover de menu-bar** compacto y denso (pantalla héroe),
  (2) **dashboard** de ventana (análisis histórico).

## Aesthetic Direction
- **Direction:** Industrial / Utilitarian — instrumento de cabina, dark-first.
- **Decoration level:** minimal. Profundidad por **escalera de superficies + hairlines
  de 1px**, nunca sombras (se ven barrosas sobre negro). **Cero gradientes en la UI**
  (el único gradiente permitido es el logo).
- **Mood:** Competencia silenciosa. Se siente como encender un instrumento, no como
  abrir un dashboard. Los números gritan, los labels susurran.
- **Reference sites:** linear.app, raycast.com, vercel.com (usage dashboard),
  grafana (gauge thresholds), btop.

## Typography
Mismas fuentes que ya carga la landing (`tokenwatch.app`) — alinea app y marca.
No se agregan fuentes.
- **Display/Hero (lecturas):** **Geist Mono** con `font-variant-numeric: tabular-nums`.
  Los números SON el héroe (34–44px, weight 500, tracking -0.02em).
- **Body / chrome / copy:** **Geist** — controles, copy explicativo en español.
- **UI/Labels:** **Geist**, small (~10–11px), UPPERCASE, tracking amplio (0.12–0.14em),
  color muted. Los labels susurran.
- **Data/Tables:** **Geist Mono** tabular para TODO dato: %, tokens, costo, timers,
  celdas, ejes, deltas. Nunca cifras proporcionales en columnas numéricas.
- **Code:** Geist Mono.
- **Loading:** `https://fonts.googleapis.com/css2?family=Geist:wght@300;400;500;600;700&family=Geist+Mono:wght@400;500;600&display=swap`
  (o self-host vía Fontsource para la app empaquetada).
- **Scale (px):** label 10–11 · body 13–14 · readout secundario 18 · sub-gauge 11–12 ·
  gauge % 30 · dashboard readout 38 · hero num 44.

## Color
- **Approach:** restrained + semántico. El color *significa estado*, no decora.
  El morado (marca) es ≤10–15% del color visible.

### Neutrals — escalera de superficies (dark, por defecto)
- `--bg` (canvas) `#0B0B0F`
- `--surface` `#121218`
- `--raised` `#171720`
- `--panel` `#101015`
- `--hover` `#1D1D27`
- `--border` `#262631` · `--border-strong` `#3A3946` · `--hairline` `rgba(255,255,255,.05)`
- `--track` (riel vacío) `#1C1C22`
- `--text` `#F4F2F8` · `--muted` `#9A96A8` · `--subtle` `#686475`

### Semantic (rampa del medidor — hace el trabajo)
- **safe** `#39D98A` · bg `rgba(57,217,138,.14)` — 0–69%, holgado
- **watch (warning)** `#F5C451` · bg `rgba(245,196,81,.14)` — 70–84%, ojo
- **danger (error)** `#FF6B6B` · bg `rgba(255,107,107,.14)` — 85–99%, frena
- **critical** `#FF3B30` · bg `rgba(255,59,48,.18)` — 100%+, sobre límite
- **info** `#60A5FA` — series de gráfico / info neutral
- Rampa colorblind-friendly (verde-ámbar/naranja-rojo). El color nunca es la única
  señal: se acompaña con posición de fill vs. ticks de umbral.

### Primary / brand (morado — demotado)
- `--brand` `#AA41F6` · `--brand-2` `#D370FF` (gradiente **solo en el logo**)
- `--brand-soft` `rgba(170,65,246,.14)` · `--brand-line` `rgba(211,112,255,.65)`
- **Uso permitido:** logo, tab/fila activa, foco, botón primario, selección.
  Razón estructural: en un sistema semáforo (verde/ámbar/rojo + azul info), el violeta
  es de los pocos tonos que NO se lee como estatus — por eso es el acento funcional
  (igual que Linear con su lavanda).
- **Uso PROHIBIDO:** como fondo, como fill de gauge, o en el medidor. El morado vive
  en el chrome, nunca en la rampa de estado.

### Pace marker (regla propia del instrumento)
- El marcador de ritmo del gauge va en **neutro de alto contraste** (`--text`, ~85%
  opacidad), NUNCA en morado — para no competir con el fill semántico (sobre todo el rojo).

### Dark / Light
- **Dark es el modo por defecto** (es un instrumento). Light existe como fallback:
  canvas `#FAFAFB`, surface `#FFFFFF`, border `#E4E4EA`, text `#16161C`, muted `#5C5A66`.
  En light, reducir saturación de la marca ~15% y mantener la misma rampa semántica.

## Spacing
- **Base unit:** 4px.
- **Density:** **compact** — la superficie de menu-bar es un instrumento; el whitespace
  de marketing desperdicia el único vistazo que tiene el usuario.
- **Scale:** 2xs(4) xs(8) sm(12) md(16) lg(24) xl(32) 2xl(48) 3xl(64).

## Layout
- **Approach:** hybrid — grid-disciplinado y denso para la app; sin composiciones
  editoriales. **Fuera las KPI cards; entran lecturas de instrumento.**
- **Popover (~360px):** top strip (logo + wordmark · `ACTUALIZADO hh:mm`) → dos
  **medidores de combustible** (`Sesión 5h`, `Semana`) como foco principal → sección
  `PRESUPUESTOS` (riel + lectura mono, con `est.` marcado) → `HOY POR PROYECTO`
  (lista rankeada densa con barra de share) → fila de comandos (toggle silenciar +
  botón Dashboard). Sin heading hero.
- **Dashboard:** lectura central grande (`12.4M · TOKENS EN RANGO`) + readouts
  secundarios en el canvas (no cards) → gráfico como panel de primera clase (línea
  luminosa sobre grid sutil, cursor tipo sonda) → tabla como **cierre de libro
  contable** (celdas mono, totales con borde superior fuerte) → editor de grupos como
  inspector lateral.
- **Max content width:** dashboard 1120–1200px.
- **Border radius:** sm 4px (rieles/chips) · md 8px (botones/inputs) · lg 12px (cards/paneles) ·
  xl 14px (popover) · full 999px (toggles/dots).
- **Gauge rail:** riel mecánico fino (8px), esquinas 2px (no "candy"), con **ticks de
  umbral en 70/85/100** y marcador de ritmo neutro. Todo medidor muestra su meta
  (nunca una barra desnuda).

## Motion
- **Approach:** minimal-functional. La telemetría **hace snap**, no anima — un fill
  que crece lee como "cargando" y mata la sensación de instrumento.
- **Easing:** enter `ease-out` · exit `ease-in` · move `ease-in-out`.
- **Duration:** micro 50–100ms · short 120–180ms · medium 180–300ms · long 300ms.
  Solo transiciones que ayudan a comprender (hover, foco, aparición del popover).
- **Excepción de firma (opcional):** al abrir el popover, el gauge puede "asentarse"
  ~600–700ms como una aguja de tacómetro. Es la única animación de dato permitida.

## Anti-slop (reglas duras)
- Sin gradientes morados de fondo (el logo es la única excepción).
- Sin gauges radiales/donut para presupuestos (modelo mental equivocado: instantáneo
  vs. tasa; y baja densidad). Usar rieles horizontales con marcador de ritmo.
- Sin barras de progreso desnudas sin meta declarada.
- Sin `-apple-system`/`system-ui` como fuente de display o body.
- Sin cifras proporcionales en columnas de datos.
- Sin fills animados / contadores easing (usar snap).
- Sin sombras para dar profundidad en oscuro (usar escalera + hairline).
- Copy en español operativo y conciso: `Sesión 5h`, `Semana`, `Presupuesto`, `Hoy`,
  `Resetea`, `Sin datos`, `Sobre límite`. Nada de lenguaje "friendly analytics".

## Decisions Log
| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-06 | Sistema de diseño inicial creado | `/design-consultation`; 4 voces (Claude main + research subagent + Codex + voz indie) convergieron en "instrumento, no dashboard" |
| 2026-07-06 | Dark-first + Geist / Geist Mono | Alinea la app con la marca pública ya comprometida en la landing (`#0b0b0f`, Geist) |
| 2026-07-06 | Morado demotado a chrma (logo/activo/foco/primario), fuera del gauge | Estructuralmente es el hue "libre" en un sistema semáforo; pero compite con el rojo si va en el medidor |
| 2026-07-06 | Marcador de ritmo en neutro, no morado | Evita colisión con el fill semántico (esp. danger) en gauges pequeños |
| 2026-07-06 | KPI cards → lecturas de instrumento; rieles con ticks 70/85/100 | El estado del límite debe ser imposible de no ver; presupuesto = agotamiento/riesgo, no logro |

<!-- Preview del sistema: ~/.gstack/projects/gsulloa-tokenwatch/designs/design-system-20260706/design-preview.html -->
