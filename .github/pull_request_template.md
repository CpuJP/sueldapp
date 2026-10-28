## Qué cambia

<!-- Describe el cambio y por qué. Enlaza el issue si existe (Closes #123). -->

## Cómo se probó

<!-- Pruebas nuevas o existentes, pasos manuales, capturas con datos de demostración. -->

## Lista de verificación

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] Todo cálculo de dinero usa `Decimal` y está en `crates/dominio` con pruebas
- [ ] No añade conexiones de red ni dependencias que las hagan
- [ ] No incluye datos reales (montos, nombres, `.ods`, `.sqlite3`, `reglas.txt`)
- [ ] Documentación y `CHANGELOG.md` actualizados si aplica
