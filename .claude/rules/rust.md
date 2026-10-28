# Reglas Rust

- Edition 2021, workspace en `Cargo.toml`. Lints comunes en `[workspace.lints]`.
- `cargo fmt` siempre; `cargo clippy --all-targets -- -D warnings` limpio.
- Sin `unsafe`. Sin `unwrap()`/`expect()` fuera de pruebas salvo invariante comentada.
- Errores con `thiserror`; `Display` en español para el usuario.
- Funciones públicas del dominio con prueba.
- Nada de abstracciones "para más adelante".
