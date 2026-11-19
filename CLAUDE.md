# TokenWatch

A macOS menu-bar app for monitoring Claude / Codex token usage per project and workspace.

## What it does (target, not yet implemented)

TokenWatch lives in the macOS menu bar (tray) and surfaces real-time and
historical token consumption for AI coding sessions. It reads usage data from
`ccusage` and from `~/.claude/projects/**/*.jsonl`, aggregates by project /
workspace, and raises alerts when configured limits are approached or exceeded.

## Current state

Scaffold only. The Tauri + React shell builds and runs, but the token-monitoring
features are NOT implemented yet. Treat this repo as the base structure.

## Structure

- `packages/app`   — Tauri 2 + React desktop app (menu-bar scaffold).
- `packages/infra` — AWS CDK infrastructure (release hosting, landing, analytics, feedback).
- `tokenwatch/`     — context folder placeholder.

## Conventions

- pnpm workspace; Node 22; Rust stable.
- `pnpm typecheck && pnpm lint && pnpm test:run` and `cargo fmt/clippy/test` must pass before landing (see PR template).

## Design System
Always read DESIGN.md before making any visual or UI decisions.
All font choices, colors, spacing, and aesthetic direction are defined there.
Do not deviate without explicit user approval.
In QA mode, flag any code that doesn't match DESIGN.md.

## Pipeline de agentes

Antes de **cualquier tarea de desarrollo** (feature, fix, chore sobre una issue),
lee `.claude/pipeline/PIPELINE.md` completo y síguelo. El hilo principal orquesta;
no implementa.

| Etapa | Agente | Modelo |
|---|---|---|
| 1. Plan | `planner` | opus |
| 2. Implementación | `coder` | sonnet |
| 3. PR draft | orquestador (push + `gh pr create --draft --base dev`) | — |
| 4. Verificación y review | `reviewer` | sonnet |
| 5. Fix (máx. 1 ronda) | `coder` en modo fix, luego `reviewer` re-verifica una vez | sonnet |

Los gates (plan, PR, hallazgos, cierre) son **automáticos**: solo hay pausa si el
planner devuelve `BLOCKED — QUESTIONS:`. Con `VERDICT: WORKS` la PR pasa a ready;
el merge sigue la regla de abajo.

Ramas: `dev` es la rama de trabajo. Una rama por issue
(`feat/<issue>-<slug>`, `fix/<issue>-<slug>` o `chore/<issue>-<slug>`) creada desde
`origin/dev`, con PR `--base dev`; nunca commitear en `dev` ni en `master`.
`master` solo recibe ramas `release/*`, y los releases los hace **siempre el
usuario a mano** (`packages/app/scripts/release.sh`): el pipeline nunca toca
`master`, no crea tags ni releases, no sube versiones y no edita `CHANGELOG.md`.

### Comando de verificación

`pnpm check` desde la raíz: `pnpm typecheck && pnpm lint && pnpm test:run` más
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test` en
`packages/app/src-tauri` (`pnpm check:rust` corre solo la parte Rust). Equivale a
CI (`.github/workflows/ci.yml`), con clippy más estricto. En un worktree nuevo,
primero `pnpm install --frozen-lockfile`.

### Regla de merge

- El orquestador (hilo principal) **puede mergear** una PR a `dev`
  (`gh pr merge <n> --squash --delete-branch`) solo si se cumplen **todas**:
  - el reviewer devolvió `VERDICT: WORKS`;
  - CI en verde (`gh pr checks <n>`: `validate` y `rust` en pass);
  - la PR no es draft.
- Tras mergear: `git fetch origin && git switch --detach origin/dev`, luego
  `pnpm install --frozen-lockfile && pnpm check`. Si se rompe, abre de inmediato
  una PR de fix.
- Los subagentes (planner, coder, reviewer) **nunca** mergean.
- Nunca force-push a `dev` ni a `master`, nunca merge con checks en rojo o
  pendientes, y nunca mergear nada a `master`.

### Cómo lanzar agentes

- Toda tarea se ejecuta con subagentes. Planeación y exploración usan el último
  modelo de Opus; implementación, review y fixes usan Sonnet (como en la tabla).
- Si el usuario pide "lanzar agentes" para issues: un agente por issue, en segundo
  plano, con `isolation: worktree` cuando corren varios en paralelo. Cada agente
  sigue este CLAUDE.md y `PIPELINE.md` completos (rama desde `origin/dev` → plan →
  código → PR draft → review → máx. 1 fix → ready → merge a `dev` según la regla).
  - Por defecto, salvo que el usuario diga otra cosa: agentes Opus con esfuerzo medio
    cuando pide orquestar agentes, y modo bypass permissions.
  - "Con las mismas reglas de hasta ahora" significa exactamente esta sección más
    Privacidad y seguridad y Design System.
  - En cada worktree nuevo el agente corre `pnpm install --frozen-lockfile` antes
    de `pnpm check`; la primera compilación de Rust tarda varios minutos.
- El handoff a cada agente debe ser explícito: número de issue, nombre de rama, ruta
  del plan, número de PR y base `dev`. Los agentes no comparten contexto.
- Tras crear cada issue o PR, agrega la entrada a la bitácora diaria (regla global),
  con proyecto `tokenwatch`.
- Si la issue es sub-issue de una issue de orquestación, actualiza su estado con el
  skill `orchestration-issues` (ver la sección de compatibilidad en `PIPELINE.md`).

## Privacidad y seguridad

TokenWatch es 100% local: sin nube ni telemetría.

- El token OAuth de Claude Code se lee del Keychain de macOS solo en
  `src-tauri/src/limits` y solo se envía a `https://api.anthropic.com/api/oauth/usage`.
  **Nunca** se loguea, se escribe a disco o SQLite, ni cruza el IPC hacia el
  frontend. Los agentes nunca ejecutan `security` ni leen el Keychain.
- De `~/.claude/projects/**/*.jsonl` solo se usan campos de uso (ids, modelo,
  tokens, timestamp, cwd/proyecto). El contenido de los mensajes nunca se loguea,
  persiste ni sube.
- Sin endpoints de red nuevos (además del de uso y el updater
  `releases.tokenwatch.gulloa.click`) ni cambios de CSP o capabilities sin
  justificarlos en el plan.
- Tests y fixtures con datos inventados (`tempfile`, `src-tauri/tests/fixtures`);
  nunca contra `~/.claude` real, la DB real (`tokenwatch.db`) ni el Keychain. Si
  hace falta validar una migración con datos reales, se hace sobre una **copia** de
  la DB en `/tmp` y solo con pedido explícito del usuario.
- Nunca pongas rutas, nombres de proyecto ni datos personales reales en issues ni
  PRs; los ejemplos van enmascarados o inventados.
- Las vulnerabilidades se reportan en privado según `SECURITY.md`, no en issues
  públicas.

## Idioma

- Commits y títulos de PR: **inglés** (Conventional Commits). El título de la PR
  es la nota de release (merge squash → `CHANGELOG.md`).
- Descripciones de PR, issues y comentarios: **español**.
- UI de la app: **español** (copy operativo de `DESIGN.md`).
