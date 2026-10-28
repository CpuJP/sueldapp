# ADR 0005 — Decisiones de migración tomadas con el usuario

- Estado: aceptada (sesión de clarificación del 2026-10-28)

Las decisiones concretas (qué deuda, qué tarjeta, qué montos) son datos personales: viven en
`docs/privado/` y `reglas.txt`, fuera del repositorio. Las reglas generales son:

1. Gastos fuera de cualquier bloque mensual: se asignan a un mes con la marca "no cuenta en el
   total", igual que en la hoja.
2. Deudas con un pago fijo sin fórmula: se tratan como crédito a cuotas; su total es la suma de sus
   cuotas (alias en `reglas.txt`). Es la única diferencia aceptada en la conciliación.
3. Tarjetas de crédito con sección propia (cupo, saldo, corte, pago). Préstamos de terceros como
   gasto con categoría Préstamos.
4. Pestañas distintas de la de nómina (cotizaciones, borradores) no se importan.

Detalle en `docs/migracion.md`.
