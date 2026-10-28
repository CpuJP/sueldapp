# ADR 0004 — Lector y escritor .ods propios

- Estado: aceptada

## Contexto

La migración necesita tres cosas de cada celda:

- el texto exacto de `office:value`, sin pasar por `f64`;
- las fórmulas, para deducir qué cuotas forman el pago de cada deuda;
- las celdas repetidas y combinadas.

## Decisión

Leer y escribir el XML del `.ods` con `zip` + `quick-xml`. Esto son unas 250 líneas.

## Alternativas

`calamine` y `spreadsheet-ods` entregan los números como `f64`.
