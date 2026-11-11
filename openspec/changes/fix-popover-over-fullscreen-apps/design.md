## Context

### Estado actual

El popover es la ventana Tauri `popover`, declarada en `packages/app/src-tauri/tauri.conf.json` con `decorations: false`, `skipTaskbar: true`, `alwaysOnTop: true` y `visible: false`. En macOS, el hook `setup` de `packages/app/src-tauri/src/lib.rs` (líneas 76-176) la convierte en `NSPanel` mediante `tauri-nspanel` (rama v2, commit `18ffb9a`, versión 2.0.1). La secuencia actual es:

1. `lib.rs:103-115`. Sobre el **`NSWindow`** todavía sin convertir, obtenido con `popover_win.ns_window()`:
   - `ns.setStyleMask(ns.styleMask() | NSWindowStyleMask::NonactivatingPanel)` (`lib.rs:109-111`);
   - `ns.setLevel(NSPopUpMenuWindowLevel)` (`lib.rs:113`).
2. `lib.rs:117`, `popover_win.to_panel()`. En `tauri-nspanel/src/raw_nspanel.rs:251-269` (`RawNSPanel::from_window`) hace `object_setClass(nswindow, RawNSPanel)`, con `RawNSPanel` como subclase de `NSPanel` registrada en runtime. Es el **mismo objeto**: solo cambia su clase.
3. `lib.rs:123-127`, `set_collection_behaviour(CanJoinAllSpaces | FullScreenAuxiliary | Stationary)`.
4. `lib.rs:132`, `set_hides_on_deactivate(false)`.
5. `lib.rs:136-173`, delegado `window_did_resign_key` con gracia de 300 ms (`popover::SHOW_GRACE`) que hace `order_out`.

El show/hide vive en `toggle_popover` (`lib.rs:345-444`). Se invoca desde `tray_macos::on_tray_clicked` (`tray_macos.rs:91-113`), el target/action propio del botón del status item que instaló #41. Para mostrar llama a `panel.show()`. En `raw_nspanel.rs:80-84` eso equivale a `makeFirstResponder(contentView)`, luego `orderFrontRegardless` y luego `makeKeyWindow`. Lo hace en el hilo principal (`run_on_main_thread`), con fallback `order_front_regardless()` + `make_key_window()`.

La app arranca como `ActivationPolicy::Accessory` (`lib.rs:53`). Solo pasa a `Regular` al abrir el dashboard (`open_dashboard`, `lib.rs:510-543`, y el item de menú "dashboard", `lib.rs:271-284`) y vuelve a `Accessory` al cerrarlo (`lib.rs:201-210`).

### Historia relevante

- **#23, julio, cambio archivado `2026-07-06-popover-fullscreen-space`.** Su primer intento fue `NSWindow` + `collectionBehavior` y falló en QA con la pantalla completa de otra app. La causa documentada en su Decisión 1b: mostrar la ventana **activaba** la app y macOS cambiaba de Space. La solución fue el `NSPanel` no-activante. El código de entonces (`git show f49f2a2:packages/app/src-tauri/src/lib.rs`) hacía `let panel = popover_win.to_panel()` y **luego** `panel.set_style_mask(1 << 7)`, es decir, el estilo se aplicaba sobre el objeto ya convertido en `NSPanel`. La QA 2.2 ("con una app en pantalla completa activa, el popover se muestra sin cambiar de Space") quedó aprobada.
- **#39, 7 nov, cambio archivado `2026-11-07-fix-session-limit-and-tray-popover`, decisión D5 (a).** Para no reemplazar la máscara completa, se cambió a `setStyleMask(styleMask() | NonactivatingPanel)` usando `objc2-app-kit`. De paso se movió la llamada **antes** de `to_panel()` (diff de `7112db2` en `lib.rs`). El comentario de `lib.rs:99-102` lo dice explícitamente: "Done on the NSWindow before it is converted to a panel".
- **#41, 7 nov.** Manejo directo del click en macOS 27. No toca la configuración del panel.

### Causa raíz

**Confianza: media-alta (~75 %).** No se puede confirmar sin ejecutar en la máquina, por eso la tarea 1 incluye un paso de confirmación con trazas.

La no-activación de un panel no es solo un bit almacenado en `styleMask`. `NSWindowStyleMaskNonactivatingPanel` está documentado como válido **solo para `NSPanel`**. Lo que impide que mostrar o hacer key el panel active la app es el efecto secundario de `-[NSPanel setStyleMask:]`: marca la ventana en el WindowServer como "prevents activation", la etiqueta privada que expone `_setPreventsActivation:`. `-[NSWindow setStyleMask:]` guarda el bit, pero no tiene ese efecto.

Como en #39 el bit se fija mientras la clase todavía es `TaoWindow`/`NSWindow`:

1. La máscara queda con el bit 7 activo. Por eso `log_panel_state` muestra `style_mask` con `0x80`, lo que es engañoso porque el bit está marcado pero no tiene efecto.
2. Después, `object_setClass` convierte el objeto en `NSPanel`, pero nadie vuelve a llamar a `-[NSPanel setStyleMask:]`.
3. Cuando `panel.show()` hace `makeKeyWindow`, el panel se comporta como activante: activa TokenWatch. Con otra app en pantalla completa en su propio Space, la activación hace que macOS cambie al Space "home" de la app activa (el escritorio), o que el panel quede ordenado fuera del Space de pantalla completa. Es exactamente el síntoma que documentó la Decisión 1b de julio para la ventana activante.
4. En el escritorio normal el síntoma no se nota, porque activar la app no cambia de Space. Por eso #39 y #41 pasaron su QA en escritorio y la regresión solo aparece con pantalla completa.

**Evidencia:**
- El orden que funcionaba es `to_panel()` y luego `set_style_mask` (`f49f2a2`, y el ejemplo oficial `tauri-nspanel/examples/fullscreen/src-tauri/src/main.rs:63-65`, que también aplica el estilo sobre el panel ya convertido).
- El orden actual es `setStyleMask` sobre `NSWindow` y luego `to_panel()` (`lib.rs:103-117`).
- El resto de la configuración relevante para pantalla completa no cambió en #39: `collectionBehavior` (`lib.rs:123-127`) y nivel 101 (`lib.rs:113`). Ya se aplicaba en julio, cuando funcionaba.

**Hipótesis descartadas o secundarias:**
- Falta de `canJoinAllSpaces`/`fullScreenAuxiliary`: ya están (`lib.rs:123-127`).
- Nivel de ventana insuficiente: `NSPopUpMenuWindowLevel` (101) está por encima de la pantalla completa y de la barra de menú (`lib.rs:113`).
- `alwaysOnTop` de `tauri.conf.json`: tao lo traduce a `NSFloatingWindowLevel` (3) al crear la ventana, pero el `setLevel(101)` posterior lo pisa. No influye.
- `hidesOnDeactivate`: ya está en `NO` (`lib.rs:132`).
- Política de activación: `Accessory`, la correcta para un agente de barra de menú. Ver Open Questions para el caso con el dashboard abierto (`Regular`).
- Click que no llega en pantalla completa: #41 instala el target/action en el propio `NSStatusBarButton`. La barra de menú de un Space de pantalla completa es el mismo status item, así que el click llega igual. El paso 1.3 de tasks.md lo verifica con el log `tray click`.
- Posicionamiento: `tray_macos::popover_origin` (`tray_macos.rs:146-164`) usa el frame de la ventana del botón del status item y la altura de la pantalla primaria para convertir a coordenadas top-left. Sirve para cualquier pantalla. No explica un popover invisible.

## Goals / Non-Goals

**Goals:**
- Con otra app en pantalla completa nativa, un click en el icono del tray muestra el popover bajo el icono, en ese mismo Space, sin activar TokenWatch y sin cambiar de Space.
- No romper nada de lo que ya funciona: primer click tras arrancar (#39), click izquierdo y derecho en macOS 27 (#41), gracia de 300 ms y debounce de 250 ms, ocultar al hacer click fuera, cerrar con un segundo click, posicionamiento y escritorio normal.
- Dejar el orden documentado en el código y un diagnóstico que distinga "bit marcado" de "panel efectivo".

**Non-Goals:**
- Migrar `tauri-nspanel` a su API nueva (`tauri_panel!`, objc2) ni cambiar de rama o commit. Requiere red y es una migración de API.
- Cambiar el comportamiento del dashboard (ventana `main`). Abrirlo desde el popover sigue activando la app y llevando al usuario a su Space, como cualquier app normal.
- Cambios en el frontend, en `tauri.conf.json`, en Windows o en Linux.
- Usar APIs privadas en el camino principal. `_setPreventsActivation:` queda solo como contingencia (Decisión 3).

## Decisions

### Decisión 1: Aplicar `NonactivatingPanel` después de `to_panel()`, sobre el objeto ya convertido en `NSPanel`

En el bloque `#[cfg(target_os = "macos")]` del `setup` (`lib.rs:81-176`) se reordena así:

1. Obtener `popover_win` (sin cambios).
2. **Convertir primero:** `let panel = popover_win.to_panel().unwrap();`.
3. **Después**, con el mismo puntero `popover_win.ns_window()`, que ya apunta a un objeto cuya clase es `RawNSPanel`, una subclase de `NSPanel`:
   ```rust
   // SAFETY: valid pointer owned by tao; after `to_panel()` its class is a
   // NSPanel subclass. Used synchronously on the main thread (setup).
   let ns: &objc2_app_kit::NSWindow = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
   ns.setStyleMask(ns.styleMask() | objc2_app_kit::NSWindowStyleMask::NonactivatingPanel);
   ns.setLevel(objc2_app_kit::NSPopUpMenuWindowLevel);
   ```
   `setStyleMask:` es un envío de mensaje Objective-C, así que el dispatch dinámico resuelve a `-[NSPanel setStyleMask:]` aunque el tipo Rust del puntero sea `NSWindow`. No hace falta castear a `NSPanel` en Rust.
4. El resto sigue igual y en el mismo orden: `set_collection_behaviour`, `set_hides_on_deactivate(false)`, delegado y `log_panel_state(&popover_win, "after_setup")`.

Se conserva el OR con la máscara existente, que fue la mejora de #39, y el nivel tipado `NSPopUpMenuWindowLevel` (101). Moverlo después de `to_panel()` es inocuo: el nivel no depende de la clase. Se mueve para tener toda la configuración del panel en un único bloque posterior a la conversión.

Hay que reemplazar el comentario de `lib.rs:99-102` por uno que explique el **porqué** del orden, para que nadie lo vuelva a invertir:

> `NonactivatingPanel` must be set AFTER `to_panel()`: only `-[NSPanel setStyleMask:]` turns the bit into real non-activation (WindowServer "prevents activation" tag). Setting it on the plain NSWindow stores the bit but the panel still activates the app, which on another app's fullscreen Space switches Spaces / hides the popover (regression from #39).

**Detalle importante: no fijar el bit antes.** Si el bit ya estuviera en la máscara antes de la conversión, `-[NSPanel setStyleMask:]` podría ver la máscara como "sin cambios" y no aplicar el efecto. Por eso hay que **eliminar** por completo la llamada previa a la conversión (`lib.rs:103-115`), no duplicarla.

*Alternativas consideradas:*
- **Volver a `panel.set_style_mask(1 << 7)` de `tauri-nspanel`.** Reemplaza la máscara completa y recibe un `i32`, cuando `NSWindowStyleMask` es `NSUInteger`. Es justo lo que #39 corrigió. Se descarta.
- **Llamar a `setStyleMask` dos veces** (sin el bit y luego con el bit) para forzar una transición. Es innecesario si el bit no se fija antes de la conversión. Queda como plan B dentro de la Decisión 3.
- **Volver a `NSWindow` + `collectionBehavior`, sin panel.** Julio ya demostró que no basta con la pantalla completa de otra app. Se descarta.
- **Migrar a la API nueva de `tauri-nspanel`** (`tauri_panel!` con `NonactivatingPanel` declarado en la clase). Es más robusto, pero es una migración de API y de dependencia, con riesgo sobre el delegado, la gracia y el debounce. Queda como seguimiento si la Decisión 3 tampoco basta.

### Decisión 2: Diagnóstico que distinga "bit marcado" de "panel efectivo"

En `packages/app/src-tauri/src/popover.rs`, `log_panel_state` (líneas 34-65) se amplía con:
- `is_panel = ns.isKindOfClass(objc2_app_kit::NSPanel::class())`. Requiere `use objc2::ClassType;` para `::class()`. `isKindOfClass` viene de `NSObjectProtocol` (`use objc2::runtime::NSObjectProtocol;`).

El campo `app_active` ya existe y es la señal clave. Si en el stage `after_show` aparece `app_active=true` mientras otra app tiene el foco a pantalla completa, el panel está activando la app y el fix no está aplicado. Si aparece `app_active=false` junto con `is_key=true` y `on_active_space=true`, el panel es no-activante y se ve en el Space activo.

No se añaden nuevos stages. Los existentes (`after_setup`, `before_show`, `after_show`, `next_tick`, `after_500ms`) bastan. El nivel de log queda en `info`; bajarlo a `debug` está pendiente del cambio anterior (D5, tarea 8.5) y no es parte de este cambio.

### Decisión 3: Contingencia con `_setPreventsActivation:`, solo si la QA falla

Si tras la Decisión 1 la QA de pantalla completa de otra app sigue mostrando `app_active=true` después de `after_show`, se aplica esto, justo después del `setStyleMask` posterior a `to_panel()`:

```rust
let sel = objc2::sel!(_setPreventsActivation:);
if ns.respondsToSelector(sel) {
    let _: () = unsafe { objc2::msg_send![ns, _setPreventsActivation: true] };
}
```

Es un selector privado de AppKit, el mismo mecanismo que usa Electron para paneles no-activantes. `respondsToSelector:` evita el crash si una versión futura lo elimina. Se documenta con un comentario. No se aplica por defecto porque la Decisión 1 restaura el comportamiento que ya funcionó en julio, sin APIs privadas.

### Decisión 4: Sin cambios en `tauri.conf.json` ni en la política de activación

- `visibleOnAllWorkspaces` y `alwaysOnTop` de Tauri no hacen falta. El `collectionBehavior` y el nivel se fijan de forma nativa y tienen precedencia. `alwaysOnTop: true` se mantiene por compatibilidad con otras plataformas.
- `macOSPrivateApi` sigue en `false`. No hace falta para nada de esto.
- La política `Accessory` se mantiene.

## Risks / Trade-offs

- **[La hipótesis es incorrecta y el reordenamiento no basta]** → La tarea 1 confirma primero con trazas (`app_active` en `after_show`). Si tras el fix sigue activando, se aplica la Decisión 3 (`_setPreventsActivation:`). Si aun así falla, el siguiente paso es migrar `tauri-nspanel`, en un cambio aparte.
- **[Al hacerse realmente no-activante, cambia el flujo de foco y `window_did_resign_key` llega en otros momentos]** → La gracia de 300 ms y el debounce de 250 ms siguen vigentes. La QA cubre el click fuera, el segundo click en el icono y el click derecho (menú). Un panel no-activante sí puede ser key (`canBecomeKeyWindow` devuelve YES en `RawNSPanel`), así que el delegado sigue recibiendo `resign key`.
- **[Teclado en el popover]** → Un panel no-activante key recibe eventos de teclado sin activar la app. Es el comportamiento estándar de los popovers de la barra de menú. Hay que verificar que el scroll y la interacción del popover siguen funcionando (QA).
- **[Regresión del primer click tras arrancar (#39)]** → `hidesOnDeactivate = NO` se mantiene. La QA incluye el caso "arrancar sin abrir el dashboard y hacer click".
- **[Con el dashboard abierto, la app está en `Regular`]** → El panel no-activante no debería activar la app ni siquiera en `Regular`. Se verifica en QA. No se cambia la política.
- **[Uso de `ns_window()` y objc2 fuera del hilo principal]** → Todo el código modificado corre en `setup`, que está en el hilo principal. `log_panel_state` ya comprueba `MainThreadMarker`.

## Migration Plan

- Cambio solo de código nativo en macOS. No hay migración de datos ni de configuración.
- Rollback: revertir el commit. El estado de ventanas persistido no se ve afectado, porque `popover` está en la denylist de window-state.
- Se publica en el siguiente release de parche (v0.3.2), con el flujo habitual de release y back-merge a `dev`.

## Open Questions

- **¿"Dashboard" significa el popover o la ventana `main`?** El usuario escribió "no aparece el dashboard" al presionar el icono de la barra. Ese click muestra el **popover**, así que se interpreta que "dashboard" se refiere al popover. Si el usuario se refería a "Abrir dashboard" estando en pantalla completa, ese flujo activa la app y muestra `main` en su Space de escritorio, que es el comportamiento estándar de macOS y queda fuera de alcance. La tarea de QA 4.9 lo observa y documenta sin cambiarlo.
- **¿Hace falta `_setPreventsActivation:` en macOS 27?** Se resuelve con la QA de la tarea 4 (Decisión 3).
- **¿Debe el popover quedarse visible al cambiar de Space con el gesto de tres dedos mientras está abierto?** Con `CanJoinAllSpaces` + `Stationary` se queda, pero el cambio de Space suele provocar `resign key` y lo oculta. Se acepta cualquiera de los dos resultados: no es parte del reporte.
