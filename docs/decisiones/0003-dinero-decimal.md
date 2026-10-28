# ADR 0003 — Dinero como Decimal, guardado como texto

- Estado: aceptada

## Decisión

Todo monto es `rust_decimal::Decimal`. En SQLite se guarda como `TEXT` y en JSON como string. El
lint `clippy::float_arithmetic` está en `deny` en todo el workspace.

## Consecuencias

- La conciliación compara valores exactos.
- La UI recibe textos ya formateados (`$ 1.234,56`) y nunca convierte a número para calcular.
