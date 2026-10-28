# Implementation Plan: App de escritorio de finanzas personales

> **Nota (2026-10-28):** la interfaz Tauri/HTML descrita aquí fue reemplazada por una interfaz
> nativa con Slint. Ver [`specs/004-interfaz-nativa`](../004-interfaz-nativa/) y la ADR 0006.

**Branch**: `001-finanzas-escritorio` | **Date**: 2026-10-28 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-finanzas-escritorio/spec.md`

## Summary

App de escritorio local que reemplaza `Ingreso-Egreso.ods`. Núcleo en Rust (dominio con
`rust_decimal`, SQLite con `rusqlite`, lector/escritor `.ods` propio con conciliación) y
interfaz Tauri 2 en HTML/CSS/JS plano con superficies de vidrio (`backdrop-filter`). Detalle de
decisiones en [research.md](research.md).

## Technical Context

**Language/Version**: Rust 1.95 (edition 2021); JS ES2022 sin bundler.

**Primary Dependencies**: `tauri` 2, `tauri-plugin-dialog`, `rust_decimal`, `rusqlite`
(bundled), `zip`, `quick-xml`, `serde`/`serde_json`, `sha2`, `thiserror`, `unicode-normalization`.

**Storage**: SQLite local (`~/.local/share/sueldapp/sueldapp.sqlite3`), migraciones con
`PRAGMA user_version`.

**Testing**: `cargo test` (unitarias en dominio, integración en almacén e importador,
conciliación contra el `.ods` real).

**Target Platform**: Linux (Fedora, WebKitGTK 2.54). Sin código atado a Linux.

**Project Type**: desktop-app.

**Performance Goals**: arranque < 2 s; cualquier acción de edición < 100 ms.

**Constraints**: sin red, montos exactos, accesibilidad AA.

**Scale/Scope**: ~500 movimientos hoy, crecimiento ~30/mes; 7 pantallas.

## Constitution Check

| Principio | Cómo se cumple | Estado |
|---|---|---|
| I. Datos locales | SQLite local, CSP `default-src 'self'`, sin plugins de red, `.gitignore` | ✅ |
| II. Migración verificable | Importador idempotente por `origen`, log por fila, prueba de conciliación | ✅ |
| III. Dominio primero | Crate `dominio` sin UI ni BD; UI solo muestra | ✅ |
| IV. Dinero exacto | `Decimal` en todo; `TEXT` en SQLite; lector `.ods` sin `f64` | ✅ |
| V. Accesibilidad | AA, teclado, reducir transparencia/movimiento | ✅ |
| VI. Simplicidad | Sin framework JS ni bundler; 4 crates | ✅ |
| VII. Puertas de calidad | fmt, clippy `-D warnings`, test | ✅ |

Re-chequeo post-diseño: sin violaciones.

## Project Structure

### Documentation (this feature)

```text
specs/001-finanzas-escritorio/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/comandos.md
├── checklists/
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml                 # workspace
crates/
├── dominio/               # tipos y reglas: mes, deuda, tarjeta, totales (sin IO)
├── almacen/               # SQLite: migraciones, repositorios
│   └── migraciones/0001_inicial.sql
├── ods/                   # lectura/escritura .ods, importador, conciliación, export CSV
│   ├── src/bin/importar.rs
│   └── tests/conciliacion.rs
└── app/                   # Tauri: comandos, ventana
    ├── tauri.conf.json
    ├── src/demo.rs        # datos ficticios para capturas
    └── ui/                # index.html, estilos.css, app.js
docs/  AGENTS.md  CLAUDE.md  README.md
```

**Structure Decision**: workspace de 4 crates. El dominio no depende de nada salvo `rust_decimal`
y `serde`; `almacen` depende de `dominio`; `ods` de ambos; `app` de todos.

## Complexity Tracking

Sin violaciones.
