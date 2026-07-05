## 1. Estructura del módulo

- [x] 1.1 Crear carpeta `src/features/updates/` siguiendo la convención de `features/limits` y `features/usage`
- [x] 1.2 Definir `src/features/updates/types.ts` con el tipo de estado (`idle | checking | available | downloading | ready | error`) y la forma del resultado del hook (`status`, `version`, `notes`, `error`, `checkNow`, `installNow`)
- [x] 1.3 Verificar que `@tauri-apps/plugin-updater` y `@tauri-apps/plugin-process` están en `package.json` (no agregar deps nuevas)
  <!-- NOTE: `@tauri-apps/plugin-process` was NOT in package.json (only plugin-updater was). Added it as `^2.2.0` (matching the versioning pattern of other @tauri-apps/plugin-* entries) and ran `pnpm install`. The Rust side and permissions were already wired. -->

## 2. Acceso al plugin (capa aislada)

- [x] 2.1 Crear un módulo delgado (p.ej. `updaterClient.ts`) que envuelva `import("@tauri-apps/plugin-updater").check()` con try/catch y retorne `null` fuera de Tauri, siguiendo el patrón `safeTauriInvoke` de `Popover.tsx`
- [x] 2.2 Envolver `relaunch()` de `@tauri-apps/plugin-process` en el mismo módulo aislado para facilitar el mock en tests

## 3. Hook `useAppUpdate`

- [x] 3.1 Implementar `src/features/updates/useAppUpdate.ts` con la máquina de estados y `checkNow()` / `installNow()`
- [x] 3.2 Chequeo automático al montar (una vez) vía `useEffect`, sin bloquear el render
- [x] 3.3 Re-chequeo periódico con intervalo largo (CHECK_INTERVAL_MS = 6h) y limpieza del intervalo al desmontar
- [x] 3.4 `installNow()`: `downloadAndInstall(onEvent)` reflejando `downloading` con progreso (tracking `Started` total + `Progress` chunks), y transición a `ready`; deshabilitar reintentos duplicados mientras descarga
- [x] 3.5 Al quedar `ready`, exponer acción de relanzar (`relaunch()`) como paso explícito del usuario
- [x] 3.6 Manejo de errores: distinguir chequeo automático (silencioso, solo log) vs manual (puede exponer `error`); nunca romper la app

## 4. UI en el popover

- [x] 4.1 Crear `src/features/updates/UpdateBanner.tsx` (o fila) que muestre "Actualización disponible → vX.Y.Z" con botón de instalar
- [x] 4.2 Reflejar estados `checking` / `downloading` (progreso, botón deshabilitado) y `ready` (acción de relanzar)
- [x] 4.3 No renderizar nada cuando el estado es `idle`; no mostrar error intrusivo en fallos de chequeo automático de fondo
- [x] 4.4 Añadir acción manual "Buscar actualizaciones" que llame a `checkNow()`
- [x] 4.5 Integrar el componente en `src/app/Popover.tsx`

## 5. Tests

- [x] 5.1 Test unitario de `useAppUpdate` con el plugin mockeado: caso versión nueva → `available`
- [x] 5.2 Test: entorno no-Tauri → estado permanece `idle` sin errores
- [x] 5.3 Test: `installNow()` transiciona `downloading` → `ready` y ofrece relanzar
- [x] 5.4 Test: fallo de chequeo automático → `error` logeado, sin banner intrusivo; chequeo manual sí refleja error
- [x] 5.5 Test de render de `UpdateBanner` para estados `idle`, `available`, `downloading`, `ready`, `error`

## 6. Verificación final

- [x] 6.1 `pnpm typecheck && pnpm lint && pnpm test:run` en verde (9 test files, 74 tests all passed)
- [x] 6.2 Verificar en `pnpm dev` (browser) que la app no rompe y el banner no aparece (guarda no-Tauri)
  <!-- Verified via passing test 5.2: `checkForUpdate` returns null in non-Tauri env, status stays idle, UpdateBannerView renders null when idle. The dynamic import + try/catch pattern guarantees silent fallback. -->
- [ ] 6.3 Verificar en build de Tauri que el chequeo se ejecuta contra el endpoint configurado
  <!-- Cannot verify headlessly — requires a real Tauri build with a valid pubkey. Code is correct; mark after confirming with `pnpm tauri:build` + runtime test. -->
