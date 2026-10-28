# Tasks: Tablero y ajustes de la alpha

- [x] T201 [US1] `dominio::estadisticas`: indicadores, acumulado y agrupar, con pruebas
- [x] T202 [US1] `texto::etiqueta_concepto` para agrupar conceptos conservando tildes
- [x] T203 [US1] Comando `tablero(desde, hasta)` en `crates/app/src/comandos.rs`
- [x] T204 [US1] Vista y gráficos SVG en `crates/app/ui/tablero.js`; paleta validada (azul/naranja; polos azul/rojo)
- [x] T205 [US2] Orden descendente por mes en `buscar_gastos`
- [x] T206 [US3] Máscara de montos `formatearEntrada` y `mascaraMonto` en `crates/app/ui/app.js`
- [x] T207 [US4] Estilo de `input.interruptor`
- [x] T208 Capturas en los 4 modos, prueba de humo y revisión visual
- [x] T209 Puertas: fmt, clippy `-D warnings`, test
- [x] T210 [US1] Selector de periodo por meses con calendario, accesos rápidos y botones por año (`tablero.js`)
- [x] T211 Filtros de Movimientos sin solaparse (`min-width: 0` en las columnas)
- [x] T212 Paginación por cursor (keyset) en `dominio::paginacion` con pruebas; `buscar_gastos` devuelve páginas de 10 por defecto
- [x] T213 Todas las listas paginadas: gastos (Movimientos, mes, cuotas, pagos) por cursor desde el núcleo; listas cortas (deudas, tarjetas, catálogos, historial, próximos pagos, ingresos) con el mismo control
- [x] T214 Sin desplazamiento horizontal en el Tablero (`min-width: 0` en grids, gráficos con `overflow: hidden`)
