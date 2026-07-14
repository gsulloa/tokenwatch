## 1. Arreglar el emparejamiento de precios

- [x] 1.1 En `packages/app/src-tauri/src/pricing.rs`, cambiar `price_row` para emparejar por familia (`model.contains("opus")` / `"sonnet"` / `"haiku"`) en lugar de `"opus-4"` / `"sonnet-4"` / `"haiku-4"`.
- [x] 1.2 Actualizar el comentario de `price_row` para reflejar que resuelve por familia (independiente de la versión mayor).

## 2. Tests

- [x] 2.1 Añadir `claude-sonnet-5` (y `claude-opus-5` / `claude-haiku-5` como guardas a futuro) a `test_all_model_variants_resolve`.
- [x] 2.2 Añadir un test de costo con `claude-sonnet-5` que verifique costo distinto de 0 y con las tarifas de la fila `SONNET`.
- [x] 2.3 Confirmar que `test_cost_unknown_model_returns_zero` (`gpt-4o`) sigue devolviendo 0 tras el cambio.

## 3. Verificación

- [x] 3.1 Ejecutar `cargo fmt`, `cargo clippy` y `cargo test` en `packages/app/src-tauri` y confirmar que pasan.
- [x] 3.2 Documentar en el PR la limitación conocida: los eventos ya persistidos con costo 0 no se recalculan (solo aplica a ingestas nuevas).
