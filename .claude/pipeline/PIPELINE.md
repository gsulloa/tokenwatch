# PIPELINE.md

Contrato de orquestación del pipeline de tokenwatch:
`planner (opus) → coder (sonnet) → PR draft → reviewer (sonnet) → [1 fix] → PR lista`.

**Solo lo lee el hilo principal.** Los subagentes no cargan este archivo: todo lo
que un agente necesita para cumplir su trabajo vive en su definición
(`.claude/agents/<agente>.md`). Si al editar esto te queda una regla que un
agente necesita, va en su archivo, no acá.

## Flujo

```text
issue → [orquestador: rama desde origin/dev] → planner → coder
      → [orquestador: push + gh pr create --draft --base dev]
      → reviewer → (blocker/major?) → coder (fix, 1 ronda) → reviewer (re-verifica, 1 vez)
      → VERDICT: WORKS → gh pr ready → (merge a dev según regla 4)
```

## Comando de verificación

`pnpm check` (desde la raíz del repo) es la verificación completa y equivale a CI:

```bash
pnpm typecheck && pnpm lint && pnpm test:run \
  && cd packages/app/src-tauri && cargo fmt --check \
  && cargo clippy --all-targets -- -D warnings && cargo test
```

`pnpm check:rust` corre solo la parte Rust. En un worktree nuevo, antes de todo:
`pnpm install --frozen-lockfile`. La primera compilación de Rust tarda varios
minutos: usa timeout de 600000 ms o `run_in_background`.

## Reglas

**1. El hilo principal orquesta, no implementa.** No edita código de la app ni
escribe planes. Solo crea la rama, lanza agentes, hace push, abre/actualiza la PR,
comenta en GitHub y mergea a `dev` bajo la regla 4.

**2. Handoffs explícitos.** Cada agente recibe en su prompt todo lo que necesita:
issue, ruta del plan, rama, número de PR y rama base (`dev`). No asumas contexto
compartido entre agentes. Las etapas son secuenciales; nunca dos en paralelo
sobre la misma issue.

**3. Una rama por issue.** `feat/<issue>-<slug>` (o `fix/`, `chore/`) creada desde
`dev` actualizado, **antes** de lanzar al planner:

```bash
git fetch origin && git switch -c <rama> origin/dev
```

Si el árbol de trabajo tiene cambios sin commitear, detente y repórtalo. Nunca se
commitea en `dev` ni en `master`. Una PR por issue (o sub-issue), siempre con
`--base dev`.

**4. Merge.** Ningún subagente mergea jamás. El orquestador (tú) solo puede mergear
a `dev` si se cumplen **todas**: el reviewer devolvió `VERDICT: WORKS`, CI en verde
(`gh pr checks <n>` con `validate` y `rust` en pass) y la PR no es draft. Comando:
`gh pr merge <n> --squash --delete-branch`. Nunca merge con checks en rojo o
pendientes, nunca force-push a `dev` ni a `master`, nunca mergear nada a `master`.

**5. Testing.** La app de escritorio (tray, popover, notificaciones, Keychain) no
se puede ejercitar de forma automática, así que los tests son la evidencia
principal: toda lógica nueva o modificada (Rust: `ingest`, `usage`, `limits`,
`budgets`, `pricing`, `db`/migraciones; TS: utilidades, hooks y componentes con
lógica en `packages/app/src/features/**`) lleva tests. Los cambios puramente
visuales no exigen test.

**6. Idioma.** Commits y títulos de PR en inglés (Conventional Commits).
Descripciones de PR, issues y comentarios en español. UI en español.

**7. Releases fuera del pipeline.** El pipeline nunca toca `master` ni ramas
`release/*`, no crea tags ni releases (`gh release`, `pnpm --filter tokenwatch release`,
`scripts/release.sh`), no sube versiones y no edita `CHANGELOG.md` (lo genera
`release.sh` desde los commits). Los releases los hace siempre el usuario a mano.
Como el merge es squash, **el título de la PR es la nota de release**: `feat` →
Added, `fix` → Fixed, `chore`/`docs`/`test` → omitido. Redáctalo para que se lea
bien como nota.

## Gates (todos con aprobación automática)

**G1 — Plan.** Cuando el planner devuelve `PLAN: <ruta>`, muestra al usuario el
resumen (5 líneas) y **continúa sin esperar**. Solo te detienes si devuelve
`BLOCKED — QUESTIONS:`, que se emite únicamente por ambigüedad de producto sin un
default razonable. En ese caso: si la sesión es interactiva, usa `AskUserQuestion`
con las preguntas (cada una con el default propuesto) y re-invoca al planner con
las respuestas; si no es interactiva, detente y devuelve las preguntas en tu
mensaje final. No contestes tú las preguntas.

**G2 — PR.** Terminado el coder (`DONE:`), sin preguntar:

```bash
git push -u origin <rama>
gh pr create --draft --base dev --head <rama> --title "<tipo>(<scope>): <resumen en inglés>" --body-file /tmp/pr-<N>-body.md
```

Título en inglés, Conventional Commits, legible como nota de release. Cuerpo en
español (escríbelo en `/tmp/pr-<N>-body.md`, fuera del repo) con estas secciones:

```markdown
## Resumen
<qué y por qué, 2-3 líneas>

## Cambios
- <lista de cambios principales>

## Cómo verificar
- `pnpm check`
- <pasos concretos tomados de los criterios del plan: tests a correr, y pasos
  manuales en `pnpm tauri:dev` para los criterios [escritorio]>

## Checklist
- [x] `pnpm typecheck` pasa
- [x] `pnpm lint` pasa
- [x] `pnpm test:run` pasa
- [x] `cargo fmt --check` sin cambios pendientes
- [x] `cargo clippy --all-targets -- -D warnings` pasa
- [x] `cargo test` pasa
- [x] La UI sigue `DESIGN.md` (o la PR no tiene cambios de UI)
- [x] El título sigue Conventional Commits y se lee como nota de release

## Riesgos y pendientes
- <hallazgos minor/nit, decisiones abiertas, criterios ⚠️ que debe probar un
  humano en la app de escritorio; "Ninguno" si no hay>

Closes #<N>
```

Marca `[x]` solo lo que el coder reportó en verde. `Closes #<N>` cierra la issue
al mergear en `dev` (rama por defecto). Reporta la URL de la PR al usuario.

**G3 — Hallazgos.** El reviewer devuelve hallazgos con severidad. Los `blocker` y
`major` se aplican automáticamente: pasan al coder en modo fix (una ronda). Los
`minor` y `nit` NO se iteran: se listan en la sección "Riesgos y pendientes" de la
PR (`gh pr edit <n> --body-file /tmp/pr-<N>-body.md`) y se terminan ahí.

**G4 — Cierre.** Con `VERDICT: WORKS` (del primer review o del re-review):
`gh pr ready <n>` y comentario en la issue (`gh issue comment <N>`) con el enlace a
la PR, qué debe mirar quien la revise y los criterios ⚠️ que requieren probar la
app de escritorio. Luego, si se cumplen las condiciones de la regla 4
(`gh pr checks <n> --watch` hasta que termine; todo en pass), mergea:

```bash
gh pr merge <n> --squash --delete-branch
git fetch origin && git switch --detach origin/dev
pnpm install --frozen-lockfile && pnpm check
```

(`--detach` porque `dev` suele estar checked out en otro worktree.) Si `pnpm check`
falla en `dev`, abre de inmediato una issue y una PR `fix/` siguiendo este mismo
pipeline. Los subagentes no mergean.

## Tope de iteración

Máximo **1 ronda de fix** y **1 re-verificación**. Si tras el fix el reviewer sigue
en `VERDICT: BROKEN`: deja la PR en draft, publica un comentario en la PR con el
resumen de los bloqueos que quedan (`gh pr comment`), y detente reportando al
usuario. No hay una segunda ronda.

## Compatibilidad con `orchestration-issues take`

Si la tarea viene de una sub-issue de orquestación, este pipeline reemplaza la
sección de pipeline que `take` espera. Reporta en la **sub-issue** (comentario
corto: qué pasó, enlace, qué sigue):

| Checkpoint | Sub-issue |
|---|---|
| Plan listo (G1, informativo: no espera respuesta) | Comentario con la ruta del plan y su resumen de 3-5 líneas |
| PR abierta (G2) | Comentario con el enlace a la PR (fila: `🔵 En curso · #<PR>`) |
| Lista para merge (G4) | Comentario "PR lista para merge" y qué mirar (fila: `🔵 En curso · #<PR> lista para merge`) |

`take` marca la tarea como hecha cuando la PR se mergea en `dev` (el humano, o el
orquestador según la regla 4) y se verifica; este pipeline no cierra la sub-issue
por su cuenta (lo hace `Closes #N` al mergear). En sesión lanzada por un
orquestador, las preguntas del planner siguen el formato de `take` (bloque
`PREGUNTAS PARA EL USUARIO`) en vez de `AskUserQuestion`.

## Invocación (plantillas literales)

Rellena los `<...>`. Cada prompt es autosuficiente.

```text
# Antes del planner: crea la rama desde dev actualizado (regla 3).
#   git fetch origin && git switch -c <rama> origin/dev

Agent(subagent_type="planner", prompt="Issue #<N> (repo gsulloa/tokenwatch): ejecuta `gh issue view <N> --comments` y planifica. Rama de trabajo: <rama> (ya creada desde origin/dev y con checkout). Base: dev. Escribe el plan en .claude/plans/<N>-<slug>.md. Contexto adicional del usuario: <texto | ninguno>.")
# → "PLAN: <ruta>": G1, muestra el resumen y sigue.
# → "BLOCKED — QUESTIONS:": ver G1. Re-invoca con "RESPUESTAS: <numeradas>".

Agent(subagent_type="coder", prompt="MODE: implement. Issue #<N> (repo gsulloa/tokenwatch). Plan: <ruta>. Rama: <rama> (ya creada y con checkout). Base: dev. Implementa el plan completo, corre `pnpm check` hasta verde, commitea en Conventional Commits (inglés) incluyendo el archivo del plan. NO hagas push ni abras PR.")
# → "DONE:": G2 (push + PR draft). "PLAN CONFLICT:": vuelve al planner con la razón (una vez);
#   si persiste, detente y reporta.

Agent(subagent_type="reviewer", prompt="PR #<n> (repo gsulloa/tokenwatch). Issue #<N>. Plan: <ruta>. Rama: <rama>. Base: dev. Verifica que funciona y revisa el diff. Publica UN comentario en la PR. <Re-verificación: findings aplicados <lista> y commit del fix <hash> | primera revisión>.")
# → "VERDICT: WORKS": G4. "VERDICT: BROKEN": G3 (si ya hubo fix, aplica el tope).

Agent(subagent_type="coder", prompt="MODE: fix. PR #<n> (repo gsulloa/tokenwatch). Issue #<N>. Plan: <ruta>. Rama: <rama> (checkout hecho). Base: dev. Aplica SOLO estos hallazgos: <lista numerada blocker/major copiada del reviewer>. Un único commit `fix:`. Corre `pnpm check`. NO hagas push.")
# → luego tú: git push; re-invoca al reviewer UNA vez (re-verificación).
```

## Lo que no se optimiza

- **El planning:** una decisión de producto mal tomada cuesta más que cualquier
  ronda de fix.
- **La privacidad de los datos locales:** el token OAuth del Keychain y el
  contenido de los JSONL de `~/.claude/projects` nunca se loguean, persisten ni
  salen de la máquina. El reviewer lo revisa en cada PR; un fallo es `blocker`.
- **La batería:** la app corre siempre en segundo plano. Un plan o diff que
  acorte intervalos de polling, relea archivos completos o agregue trabajo
  periódico sin justificar es hallazgo `major`.
