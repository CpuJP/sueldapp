# AGENTS.md

Fuente única de instrucciones para cualquier agente que trabaje en este repositorio.

## Qué es

Sueldapp: app de escritorio local para finanzas personales (sueldo, gastos mensuales, deudas a
cuotas, tarjetas de crédito). Reemplaza la hoja `Ingreso-Egreso.ods`, de la que importa todos los
datos con conciliación exacta. Un usuario, un equipo, sin red. Moneda: COP.

## Stack

- Rust estable (workspace), `rust_decimal` para dinero, SQLite con `rusqlite` (bundled).
- Tauri 2 con interfaz HTML/CSS/JS plano (sin npm ni bundler) en `crates/app/ui/`.
- Lectura/escritura `.ods` propia con `zip` + `quick-xml` (sin `f64`).

## Estructura

```text
crates/dominio/   tipos y reglas de negocio puras (sin IO). Aquí vive todo cálculo de dinero.
crates/almacen/   SQLite: migraciones (migraciones/NNNN_*.sql) y repositorios.
crates/ods/       lector/escritor .ods, importador, conciliación, export CSV, binario `importar`.
crates/app/       Tauri: comandos (src/comandos.rs) e interfaz (ui/).
specs/            artefactos de Spec Kit (spec, plan, tasks…).
docs/             análisis de la hoja, arquitectura, guía de usuario, migración, ADRs.
```

## Comandos

```bash
cargo build --workspace
cargo test --workspace                        # incluye conciliación contra Ingreso-Egreso.ods
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p sueldapp                         # ejecutar la app
cargo run -p sueldapp-ods --bin importar -- Ingreso-Egreso.ods   # migrar desde terminal
SUELDAPP_DB=/ruta/demo.sqlite3 SUELDAPP_DEMO=1 cargo run -p sueldapp   # datos ficticios
SUELDAPP_DB=/tmp/demo.sqlite3 SUELDAPP_DEMO=1 SUELDAPP_CAPTURAS=docs/capturas cargo run -p sueldapp --features capturas  # capturas + prueba de humo
```

La prueba de conciliación busca el `.ods` en la raíz o en `SUELDAPP_ODS`. Sin el archivo falla;
con `SUELDAPP_SIN_ODS=1` se omite (CI y equipos sin los datos). Usa las reglas personales de
`reglas.txt` en la raíz (o `SUELDAPP_REGLAS`) si existen.

Instaladores: `(cd crates/app && npx @tauri-apps/cli@2.12.2 build)`. CI y releases en
`.github/workflows/` (Linux y Windows, x64 y ARM64); ver `docs/RELEASES.md`.

## Convenciones Rust

- Dinero: siempre `Decimal` (alias `Dinero`). Prohibido `f32`/`f64` (lint `float_arithmetic`).
  En SQLite, montos como `TEXT`. En JSON, montos como string.
- Errores: `thiserror` en librerías; mensajes para el usuario en español sencillo.
- Sin `unwrap()`/`expect()` fuera de pruebas, salvo invariantes documentadas.
- Nombres de dominio en español (`gasto`, `deuda`, `mes`), igual que la UI.
- La UI no calcula dinero: muestra lo que devuelve el núcleo.

## Flujo Spec Kit

constitución → specify → clarify → plan → checklist → tasks → analyze → implement. La spec activa
está en `specs/001-finanzas-escritorio/`. `tasks.md` es la fuente de verdad del progreso: marca
cada tarea al terminarla. Nada fuera de la spec aprobada; ideas a `docs/ideas.md`.

## Privacidad

El repositorio es público. Nunca subas a git ni copies a `docs/` datos reales: `.ods`, `.sqlite3`,
`logs/`, exportaciones, `reglas.txt` ni `docs/privado/`. Tampoco montos o nombres reales en pruebas,
comentarios o specs: usa valores inventados. Las reglas con nombres propios (tarjetas, alias de
deudas, categorías personales) van en `reglas.txt`, nunca en el código. Las capturas usan el modo demo.

## Definición de terminado

1. La tarea está marcada en `tasks.md`.
2. `cargo fmt --check`, `cargo clippy -- -D warnings` y `cargo test` en verde.
3. Si toca dinero: hay prueba en `dominio` o en la conciliación.
4. Si toca la UI: probada con teclado, en claro, oscuro y reducir transparencia.
5. Documentación afectada actualizada.
