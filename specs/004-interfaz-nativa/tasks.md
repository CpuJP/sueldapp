# Tasks: Interfaz nativa (Slint)

**Plan**: [plan.md](plan.md)

## Fase 1 — Base

- [x] T001 Quitar Tauri, la UI web y `tauri.conf.json`; añadir `slint`, `slint-build`, `rfd`, `png`
- [x] T002 `ui/tema.slint` con tokens claro/oscuro/sólido y `ui/componentes.slint`
- [x] T003 Ventana con barra lateral, fondo y atajos Alt+1…8

## Fase 2 — Pantallas

- [x] T004 Tablero (calendario, presets, cifras, gráficos con tooltip)
- [x] T005 Resumen (mes y año), Movimientos con filtros y paginación por cursor
- [x] T006 Deudas, Tarjetas, Catálogos, Importar y exportar, Ajustes (con «Hecho con Slint»)

## Fase 3 — Interacción

- [x] T007 Diálogos de edición, confirmación al borrar, avisos con deshacer
- [x] T008 Máscara de montos (`mascara.rs`, con pruebas)
- [x] T009 Ajustes persistentes en `ajustes.txt`

## Fase 4 — Verificación

- [x] T010 Capturas en los 4 modos con `take_snapshot` y comparación con la versión web
- [x] T011 Prueba de humo (crear, marcar pagado, borrar y deshacer) y pruebas de `vistas.rs`
- [x] T012 Medir memoria: 52 MiB (GPU) / 36 MiB (software) con datos reales, antes ~210 MiB

## Fase 5 — Distribución y documentación

- [x] T013 CI sin WebKit; release con `cargo-packager` y `cargo-generate-rpm`
- [x] T014 ADR 0006, README (atribución Slint), AGENTS, arquitectura, FORKS, RELEASES, CHANGELOG
