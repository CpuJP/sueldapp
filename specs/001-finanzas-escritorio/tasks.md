---
description: "Task list for App de escritorio de finanzas personales"
---

# Tasks: App de escritorio de finanzas personales

**Input**: Design documents from `/specs/001-finanzas-escritorio/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/comandos.md

**Tests**: Requeridos por la constitución (principios II, III y VII): dominio con pruebas antes que
UI y conciliación automática.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: se puede hacer en paralelo (archivos distintos, sin dependencias)
- **[Story]**: historia de usuario (US1…US7)

## Phase 1: Setup

- [x] T001 Crear workspace `Cargo.toml` con crates `crates/dominio`, `crates/almacen`, `crates/ods`, `crates/app` y `rust-toolchain`/lints comunes
- [x] T002 [P] Crear `AGENTS.md`, `CLAUDE.md` (importa `@AGENTS.md`), `README.md` y `docs/ideas.md`
- [x] T003 [P] Crear `.claude/settings.json` (permisos y hook `cargo fmt` tras editar `.rs`), `.claude/agents/{analista-datos,arquitecto-rust,disenador-ui,revisor,tester}.md` y `.claude/rules/{rust,ui,datos-financieros,git}.md`
- [x] T004 [P] Inicializar git con `.gitignore` verificado (ningún `.ods`, SQLite, log ni export rastreado)

## Phase 2: Foundational (bloquea todas las historias)

- [x] T005 Tipos de dominio (`Dinero` = `Decimal`, `Mes`, `Gasto`, `Ingreso`, `Estado`, `Deuda`, `Tarjeta`) en `crates/dominio/src/lib.rs`
- [x] T006 Reglas del mes y del año (Total, Pagado, Ingreso, Restante, Restante pagado, exclusión `fuera_de_bloque`) con pruebas en `crates/dominio/src/calculos.rs`
- [x] T007 Reglas de deuda (pagado, pendiente, progreso, cuotas, fin estimado) y de tarjeta (disponible) con pruebas en `crates/dominio/src/calculos.rs`
- [x] T008 Formato COP `$ 1.234.567,89` y parseo de montos escritos por el usuario, con pruebas, en `crates/dominio/src/formato.rs`
- [x] T009 Normalización de texto (minúsculas, sin tildes) y raíz de concepto para vínculos, con pruebas, en `crates/dominio/src/texto.rs`
- [x] T010 Migración `crates/almacen/migraciones/0001_inicial.sql` (tablas de data-model.md + catálogos iniciales) y runner con `PRAGMA user_version`, WAL y `foreign_keys` (FR-018) en `crates/almacen/src/lib.rs`
- [x] T011 Repositorios CRUD (meses, gastos, ingresos, deudas, tarjetas, catálogos, importaciones) con upsert por `origen` y pruebas en memoria en `crates/almacen/src/repo.rs`

**Checkpoint**: dominio y almacén probados.

## Phase 3: US1 — Importar la hoja sin perder nada (P1) 🎯 MVP

**Goal**: importar `Ingreso-Egreso.ods` con log y conciliación exacta.
**Independent Test**: `cargo test -p sueldapp-ods --test conciliacion` en verde contra el `.ods` real.

- [x] T012 [US1] Lector de `.ods` (zip + quick-xml): celdas con texto, valor exacto, fórmula, columnas repetidas y combinadas en `crates/ods/src/lectura.rs`
- [x] T013 [US1] Parser de fórmulas simple: referencias `[.L163]` y constantes sumadas/restadas en `crates/ods/src/formula.rs`
- [x] T014 [US1] Extractor de `Nómina`: franjas por año, bloques mensuales, ingresos (sueldo, prima, desglose), gastos, fuera de bloque, deudas, valores esperados de la hoja en `crates/ods/src/hoja.rs`
- [x] T015 [US1] Reglas de mapeo: estados, categorías, tarjetas de crédito, vínculo de cuotas por fórmula y por nombre, deudas sin fórmula, omisión de otras pestañas, log por fila en `crates/ods/src/importar.rs`
- [x] T016 [US1] Conciliación (mes, año, deuda; lista blanca de deudas sin fórmula) en `crates/ods/src/conciliar.rs`
- [x] T017 [US1] Escritura transaccional en SQLite + registro en `importaciones` (huella SHA-256; sin escritura parcial ante error) en `crates/ods/src/importar.rs`
- [x] T018 [US1] Prueba de integración contra el `.ods` real: 0 diferencias, idempotencia (2 importaciones = mismos conteos), huella del original intacta, en `crates/ods/tests/conciliacion.rs`
- [x] T019 [P] [US1] Binario `crates/ods/src/bin/importar.rs` (importa y escribe el log en `logs/`)

## Phase 4: US2 — Resumen del mes (P1)

**Independent Test**: un mes importado muestra Total, Pagado y Restante iguales a la hoja.

- [x] T020 [US2] App Tauri mínima: `crates/app/tauri.conf.json` (CSP `default-src 'self'`, sin red), `crates/app/src/main.rs`, base SQLite en directorio de datos
- [x] T021 [US2] Comandos `resumen_mes`, `resumen_anio`, `listar_meses` en `crates/app/src/comandos.rs`
- [x] T022 [US2] Sistema visual: tokens claro/oscuro, vidrio, reducir transparencia/movimiento, tipografía y cifras tabulares en `crates/app/ui/estilos.css`
- [x] T023 [US2] Estructura y navegación (barra lateral, router, teclado, diálogos accesibles, avisos con deshacer) en `crates/app/ui/index.html` y `crates/app/ui/app.js`
- [x] T024 [US2] Pantalla Resumen (mes y año, próximos pagos, estado vacío) en `crates/app/ui/app.js`

## Phase 5: US3 — Registrar y editar movimientos (P1)

**Independent Test**: marcar una cuota pendiente como Pagado sube Pagado del mes y baja el pendiente de su deuda.

- [x] T025 [US3] Comandos de gastos e ingresos (buscar, guardar, borrar, restaurar) y meses (guardar, borrar) en `crates/app/src/comandos.rs`
- [x] T026 [US3] Pantalla Movimientos: búsqueda sin tildes, filtros, formulario crear/editar con validación, borrado con confirmación y deshacer en `crates/app/ui/app.js`

## Phase 6: US4 — Deudas (P2)

- [x] T027 [US4] Comandos de deudas en `crates/app/src/comandos.rs`
- [x] T028 [US4] Pantalla Deudas (total, pagado, pendiente o "pagado de más", progreso, cuotas, fin estimado, CRUD) en `crates/app/ui/app.js`

## Phase 7: US4b — Tarjetas de crédito (P2)

- [x] T029 [US4b] Comandos de tarjetas en `crates/app/src/comandos.rs`
- [x] T030 [US4b] Pantalla Tarjetas (cupo, saldo, disponible, corte, pago, pagos vinculados, CRUD) en `crates/app/ui/app.js`

## Phase 8: US5 — Catálogos editables (P2)

- [x] T031 [US5] Comandos de catálogos con reasignación al borrar en `crates/almacen/src/repo.rs` y `crates/app/src/comandos.rs`
- [x] T032 [US5] Pantalla Catálogos (estados con "cuenta como pagado", categorías, fuentes) en `crates/app/ui/app.js`

## Phase 9: US6 — Exportar y volver a importar (P2)

- [x] T033 [US6] Escritor `.ods` propio (una hoja por tabla, montos desde `Decimal`) e importador de ese formato en `crates/ods/src/escritura.rs` y `crates/ods/src/importar.rs`
- [x] T034 [P] [US6] Exportación CSV (un archivo por tabla, UTF-8, montos con punto decimal) en `crates/ods/src/csv.rs`
- [x] T035 [US6] Prueba ida y vuelta: exportar → importar en base vacía → mismos totales, en `crates/ods/tests/ida_vuelta.rs`
- [x] T036 [US6] Comandos `importar_ods`, `exportar_ods`, `exportar_csv`, `historial_importaciones` + pantalla Importar/Exportar con informe de conciliación en `crates/app/src/comandos.rs` y `crates/app/ui/app.js`

## Phase 10: US7 — Apariencia y accesibilidad (P3)

- [x] T037 [US7] Ajustes: reducir transparencia (persistido), respeto a `prefers-reduced-motion` y `prefers-color-scheme` en `crates/app/ui/app.js` y `crates/app/ui/estilos.css`
- [x] T038 [US7] Modo demo (`SUELDAPP_DB`, `SUELDAPP_DEMO=1`, `SUELDAPP_TEMA`) con datos ficticios para capturas en `crates/app/src/demo.rs`
- [x] T039 [US7] Capturas de cada pantalla en claro, oscuro y reducir transparencia; revisar contraste AA y patrones prohibidos; corregir en `docs/capturas/`

## Phase 11: Polish

- [x] T040 [P] `docs/arquitectura.md` (con diagrama), `docs/migracion.md`, `docs/guia-de-usuario.md` (con capturas) y ADRs en `docs/decisiones/`
- [x] T041 Importar el `.ods` real en la base de la app; comprobar arranque < 2 s (SC-004) y edición < 1 s (SC-003) con datos migrados
- [x] T042 Puertas finales: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`; verificar que no hay dependencias de red en tiempo de ejecución (`cargo tree`) ni orígenes externos en la CSP/UI (Constitución I)

## Dependencies & Execution Order

- Setup → Foundational → US1 → US2 → US3 → (US4, US4b, US5, US6 en cualquier orden) → US7 → Polish.
- US2 necesita datos (US1) para su prueba independiente; US3 necesita las pantallas base de US2.

## Parallel Example

```text
T002, T003, T004 en paralelo (archivos distintos).
T019 en paralelo con T020 una vez terminado T017.
T034 en paralelo con T033.
```

## Implementation Strategy

MVP = Phase 1–5 (importar, ver y editar). Luego deudas, tarjetas, catálogos, exportación y pulido
visual. Cada fase termina con las puertas de calidad en verde.
