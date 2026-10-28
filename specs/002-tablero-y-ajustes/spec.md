# Feature Specification: Tablero de estadísticas y ajustes de la alpha

**Feature Branch**: `002-tablero-y-ajustes`

**Created**: 2026-10-28

**Status**: Implementado

**Input**: Comentarios del usuario tras probar la alpha con datos reales:

- Movimientos del más reciente al más antiguo.
- Montos con separador de miles automático, coma decimal y como máximo 2 decimales.
- Interruptores mal dibujados.
- Un módulo "Tablero", encima de Resumen, con estadísticas y gráficos dinámicos de todo el dinero:
  ingresos y gastos por mes, ahorro y faltantes.

## User Scenarios & Testing

### US1 — Tablero (P1)

Abro la app y veo el tablero. Muestra cuánto me sobró o faltó en el periodo, mis ingresos y
gastos, mi tasa de ahorro, los meses con sobrante y con faltante, los promedios, el mejor y el peor
mes, la deuda pendiente total y estos gráficos:

- ingresos y gastos por mes;
- lo que sobra o falta cada mes;
- el saldo acumulado;
- el gasto por destino (deudas, tarjetas, otros), por categoría y por estado;
- los conceptos con más gasto, los gastos más grandes, las deudas pendientes y los ingresos por
  fuente.

Puedo elegir el periodo: hasta hoy, últimos 12 meses, este año o todo (con la proyección).

**Acceptance**:

1. Al pasar el puntero sobre un mes, veo sus valores.
2. Todos los valores están también en una tabla.
3. Los totales coinciden con la suma de los resúmenes mensuales.

### US2 — Orden de movimientos (P1)

Los movimientos se listan del mes más reciente al más antiguo. Dentro de cada mes se mantiene el
orden de la hoja.

### US3 — Entrada de montos (P1)

Al escribir `4200000` el campo muestra `4.200.000`. La coma es el separador decimal y no se pueden
escribir más de 2 decimales. El punto del teclado numérico se convierte en coma.

### US4 — Interruptores (P2)

Los interruptores de Ajustes se dibujan como un riel con su perilla, en los tres modos.

## Requirements

- **FR-201**: Las estadísticas se calculan en `dominio::estadisticas`, con pruebas.
- **FR-202**: Los gráficos validan la paleta con la herramienta de la skill dataviz, usan un solo
  eje, tienen leyenda cuando hay 2 o más series y ofrecen tabla de datos.
- **FR-203**: Los gastos fuera de bloque no cuentan en las estadísticas, igual que en los totales.
- **FR-204**: El tablero es la primera sección (Alt + 1) y la vista inicial.

## Out of scope

Presupuestos, metas de ahorro y predicciones (ver `docs/ideas.md`).
