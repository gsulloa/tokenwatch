## 1. Backend: enumeración completa de buckets

- [x] 1.1 En `packages/app/src-tauri/src/usage/mod.rs`, añadir un helper `all_buckets_in_range(conn, params, strftime_fmt) -> anyhow::Result<Vec<String>>` que ejecute un CTE recursivo en SQLite y devuelva las etiquetas de bucket completas y ordenadas del rango efectivo.
- [x] 1.2 En el helper, resolver los bounds `lo`/`hi`: usar `params.since`/`params.until` si vienen definidos; si no, derivarlos de `MIN(timestamp)`/`MAX(timestamp)` con el mismo `where_clause` que el query de datos (convertidos a hora local con `'localtime'`).
- [x] 1.3 Implementar el arranque y paso del CTE por granularidad, anclado a datetime local y formateando cada paso con `strftime_fmt`: hora (`+1 hour`), día (`+1 day`), semana (inicio de semana local `+7 days`), mes (`start of month` `+1 month`).
- [x] 1.4 Hacer inclusivo el bucket que contiene `hi` (condición de parada comparada contra el inicio del bucket de `hi`, no contra `hi` exacto).
- [x] 1.5 Acotar defensivamente las iteraciones del CTE con un límite duro razonable para evitar un CTE runaway ante bounds inválidos.

## 2. Backend: integrar en query_series_inner

- [x] 2.1 Reemplazar el bloque de derivación de `buckets` (actuales líneas ~334-342, que dedupean las filas del query) por el resultado de `all_buckets_in_range`.
- [x] 2.2 Verificar que `series_names`, `value_map` y el ensamblado con `unwrap_or(0.0)` (líneas ~358-373) quedan sin cambios y ahora rellenan 0 contra la lista completa.
- [x] 2.3 Mantener el early-return de estado vacío cuando no hay ningún evento en el rango (no fabricar gráfico plano en 0).

## 3. Tests (Rust)

- [x] 3.1 Actualizar `test_empty_bucket_filling` para reflejar el nuevo comportamiento: buckets intermedios del rango presentes con valor 0 (p.ej. día 2 aparece entre día 1 y día 3).
- [x] 3.2 Añadir test bucket=hora con `since`/`until` de 24h y eventos en pocas horas → la respuesta contiene 24 buckets y las horas sin eventos son 0.
- [x] 3.3 Añadir test de bucket final inclusivo (evento/rango que termina dentro del último bucket → ese bucket se incluye).
- [x] 3.4 Añadir test de rango que cruza medianoche y (para semana) fin de año, verificando match exacto de etiquetas entre enumeración y datos.

## 4. Verificación

- [x] 4.1 Ejecutar `cargo fmt`, `cargo clippy` y `cargo test` en `packages/app/src-tauri` sin errores.
- [x] 4.2 Ejecutar `pnpm typecheck && pnpm lint && pnpm test:run`.
- [x] 4.3 Verificar en la app: en el dashboard, preset "últimas 24 horas" muestra las 24 horas en el eje X con 0 en las horas sin uso; comprobar también presets de día/semana/mes.
