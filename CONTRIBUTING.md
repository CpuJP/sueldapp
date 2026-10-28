# Cómo contribuir a Sueldapp

¡Gracias por tu interés! Sueldapp es una app de finanzas personales **local y privada**. Toda
contribución debe respetar sus principios (ver [la constitución](.specify/memory/constitution.md)).

## Principios que no se negocian

1. **Datos locales.** Nada de red en tiempo de ejecución: ni telemetría, ni analítica, ni CDNs.
2. **Dinero exacto.** Montos con `rust_decimal::Decimal`; nunca `f32`/`f64` (hay un lint que lo impide).
3. **Dominio con pruebas.** Todo cálculo vive en `crates/dominio` y tiene prueba. La interfaz no calcula dinero.
4. **Sin datos reales en el repositorio.** Ni `.ods`, ni `.sqlite3`, ni `reglas.txt`, ni montos o nombres
   reales en pruebas, capturas o issues. Usa el modo demo (`SUELDAPP_DEMO=1`).
5. **Simplicidad.** Solo lo que pide la spec aprobada; las ideas van a [`docs/ideas.md`](docs/ideas.md).

## Preparar el entorno

```bash
# Fedora
sudo dnf install webkit2gtk4.1-devel libsoup3-devel gtk3-devel
# Debian/Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libxdo-dev libssl-dev librsvg2-dev
# Rust estable: https://rustup.rs
```

En Windows basta con Rust (MSVC) y WebView2, que ya viene en Windows 10/11.

## Flujo de trabajo

1. Abre un issue antes de un cambio grande, para acordar el alcance.
2. Haz un fork y crea una rama: `git switch -c feat/mi-cambio`.
3. Para funciones nuevas usamos [Spec Kit](https://github.com/github/spec-kit): spec → plan → tasks
   en `specs/NNN-nombre/`. Para arreglos pequeños basta con el PR.
4. Antes de abrir el PR:
   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   SUELDAPP_SIN_ODS=1 cargo test --workspace
   ```
5. Si tocas la interfaz, revisa claro, oscuro y "reducir transparencia", y que todo se use con teclado.
   Para capturas: `SUELDAPP_DB=/tmp/demo.sqlite3 SUELDAPP_DEMO=1 SUELDAPP_CAPTURAS=/tmp/capturas cargo run -p sueldapp --features capturas`.
6. Mensajes de commit con [Conventional Commits](https://www.conventionalcommits.org/es/) en español:
   `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`.
7. Actualiza `CHANGELOG.md` en la sección "Sin publicar".

## Estructura

Ver [`AGENTS.md`](AGENTS.md) y [`docs/arquitectura.md`](docs/arquitectura.md).

## Código de conducta

Este proyecto sigue el [Código de conducta](CODE_OF_CONDUCT.md). Al participar, te comprometes a respetarlo.
