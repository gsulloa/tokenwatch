## Context

TokenWatch es una app menú-bar de Tauri 2 + React. El backend del updater ya está listo: `tauri-plugin-updater` registrado en `src-tauri/src/lib.rs` (`.plugin(UpdaterBuilder::new().build())`), config en `tauri.conf.json` (`plugins.updater.endpoints = ["https://releases.tokenwatch.gulloa.click/latest.json"]`), y permisos `updater:default` + `process:allow-restart` en `src-tauri/capabilities/default.json`. Las deps JS `@tauri-apps/plugin-updater` y `@tauri-apps/plugin-process` ya están en `package.json`.

Lo único que falta es la capa de frontend: nada en `src/` invoca `check()`. El popover del menú-bar (`src/app/Popover.tsx`) ya usa un patrón `safeTauriInvoke` que degrada limpio en entornos no-Tauri (dev en browser, tests). Ese patrón es la referencia para el guardado de este feature.

## Goals / Non-Goals

**Goals:**
- Chequear actualizaciones al arrancar y bajo demanda, sin bloquear la UI.
- Notificar en el popover cuando hay versión nueva y permitir instalar + relanzar.
- Degradar silenciosamente fuera de Tauri (dev/test) igual que `safeTauriInvoke`.
- Cobertura de tests unitarios (hook + UI) con mocks del plugin, estilo Vitest de `features/usage`.

**Non-Goals:**
- Generar/rotar la `pubkey` de firma (hoy placeholder en `tauri.conf.json`) — es tarea de release/infra, fuera de este cambio de frontend.
- Cambiar el pipeline de CI/CD, los manifiestos (`latest.json`) ni la infra de hosting.
- Actualizaciones silenciosas/forzadas sin consentimiento del usuario. El usuario siempre decide instalar.
- Cambios en el backend Rust (el plugin ya está montado).

## Decisions

**Ubicación: nuevo módulo `src/features/updates/`.** Sigue la convención de feature-folders del repo (`features/limits`, `features/usage`). Contiene el hook, el componente de UI, tipos y tests.

**Hook `useAppUpdate` como única fuente de verdad de estado.** Máquina de estados simple: `idle | checking | available | downloading | ready | error`. Expone `{ status, version, notes, error, checkNow(), installNow() }`. Alternativa descartada: dispersar la lógica en el componente — dificulta test y reuso.

**Import dinámico + guarda no-Tauri.** Igual que `safeTauriInvoke` en `Popover.tsx`: `const { check } = await import("@tauri-apps/plugin-updater")` dentro de try/catch; si falla (browser/test), se omite y queda `idle`. Evita romper `pnpm dev` en navegador y los tests de Vitest.

**Chequeo al montar + intervalo ligero.** Un chequeo al arrancar (via `useEffect` una vez) y un re-chequeo periódico de baja frecuencia. Se elige un intervalo largo (p.ej. cada pocas horas) para no golpear el endpoint; el valor concreto se fija en implementación. El chequeo manual comparte el mismo camino que el automático.

**Instalación con `downloadAndInstall()` + `relaunch()`.** El objeto `Update` devuelto por `check()` expone `downloadAndInstall(onEvent)`; se usa su callback de progreso para reflejar `downloading`. Al terminar (`ready`), se ofrece relanzar con `relaunch()` de `@tauri-apps/plugin-process` (permiso `process:allow-restart` ya concedido). El relanzamiento es acción explícita del usuario, no automático.

**UI en el popover, no en el dashboard.** El popover del menú-bar es el punto de contacto frecuente y liviano; ahí encaja un banner/fila discreto "Actualización disponible → vX.Y.Z". El dashboard queda fuera de alcance para no recargarlo.

**Errores diferenciados por origen.** El hook distingue chequeo automático (de fondo, silencioso ante error) vs. manual (puede reflejar error en UI). Se registra siempre vía el logging existente; los automáticos no muestran banner de error.

## Risks / Trade-offs

- **`pubkey` es placeholder en prod** → sin la clave real, la verificación de firma del updater fallará en producción. Mitigación: este cambio solo cubre el frontend; se documenta como dependencia externa y el manejo de error del hook evita que un fallo de firma rompa la app.
- **Endpoint caído / offline** → chequeo automático falla. Mitigación: fallo silencioso en chequeos de fondo + reintento en el próximo ciclo; solo el chequeo manual muestra error.
- **Mock del plugin en tests** → el import dinámico puede complicar el mocking en Vitest. Mitigación: aislar el acceso al plugin en una función/módulo delgado fácil de mockear, como hace `safeTauriInvoke`.
- **Ruido de re-chequeos** → intervalos cortos golpean el endpoint sin valor. Mitigación: intervalo largo y un solo chequeo garantizado al arranque.

## Migration Plan

Feature aditivo, sin migración de datos ni breaking changes. Deploy con el próximo release. Rollback = revertir el módulo `features/updates/` y su integración en `Popover.tsx`; el resto de la app no depende de él.

## Open Questions

- Frecuencia exacta del re-chequeo periódico (¿cada 4h? ¿6h?) — decisión menor a fijar en implementación.
- ¿Mostrar las notas de release completas en el popover o solo la versión + link? Depende del espacio del popover; por defecto, versión + acción.
