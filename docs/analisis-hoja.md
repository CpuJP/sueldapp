# Formato de la hoja original (`Ingreso-Egreso.ods`)

Este documento describe la **estructura** de la hoja de cálculo que Sueldapp sabe importar. No
contiene datos: el análisis con cifras y nombres reales es personal y vive en `docs/privado/`
(no versionado).

## Resumen del formato

| Aspecto | Valor |
|---|---|
| Pestaña con datos | `Nómina` (las demás se omiten y quedan en el log) |
| Moneda | Peso colombiano (formato `es-CO`, `$ 1.234.567,89`) → COP |
| Montos | Números (`office:value`) o fórmulas aritméticas simples |

## Pestaña `Nómina`

La pestaña tiene dos zonas independientes.

### Zona 1 — Control de deudas (A7:E…)

Encabezados en la fila 7 (`Item | Pago | Total | Pendiente`) y una fila por deuda desde la fila 8,
hasta la primera fila vacía en la columna A.

| Columna | Encabezado | Contenido |
|---|---|---|
| A | Item | Nombre de la deuda |
| B | Pago | Total pagado. Fórmula que **suma celdas concretas de cuotas** de los bloques mensuales, más a veces una constante (ajuste) |
| C | Total | Monto total de la deuda (número o fórmula aritmética, p. ej. `=250000*6`) |
| D | Pendiente | `=C − B` |
| E | (sin título) | `OK` cuando la deuda está saldada |

### Zona 2 — Control de sueldo por mes

Columnas por año, en franjas de 4 columnas. La fila 4 tiene el año de cada franja (F4, K4, P4, U4…):

| Franja | Concepto | Monto | Estado | SubTotal |
|---|---|---|---|---|
| 1.ª | F | G | H | I |
| 2.ª | K | L | M | N |
| … | +5 columnas | | | |

Cada mes es un **bloque** vertical:

1. Encabezado `Sueldo <Mes> <Año>` (celda combinada). Una celda `OK` a la derecha marca el mes
   como **cerrado**.
2. Fila de títulos: `Concepto | Gastos fijos | Estado`.
3. Filas de gasto: concepto, monto, estado.
4. `Total` = `SUM` de todos los montos del bloque; a su derecha, `SubTotal` = `SUMIF(Estado="OK")`.
5. `Sueldo` = ingreso del mes (número o fórmula; puede sumar la celda `Prima`).
6. `Restante` = `Sueldo − Total`; a su derecha, `SubRestante` = `Sueldo − SubTotal`.
7. Opcional (junio y diciembre): fila `Prima`.

Al final de cada franja: `TOTAL PAGO <año>`, `TOTAL GASTO <año>` y `TOTAL RESTANTE <año>`.

Valores de `Estado`: `OK` (o `Ok`), `NO`, `APLAZADO`, `PENDIENTE` o vacío. Los colores de fondo solo
acompañan al estado.

## Modelo de datos implícito

- **Periodo** (mes/año): sueldo, prima opcional, marca "cerrado".
- **Gasto**: periodo, concepto, monto, estado.
- **Deuda**: nombre, total, ajuste fijo de pago, marca "saldada".
- **Cuota**: gasto vinculado a una deuda; el vínculo solo existe en las fórmulas de la columna B.
- **Ingreso**: sueldo y prima por periodo.

## Reglas de negocio

- R1. `Total del mes` = suma de todos sus gastos, sea cual sea su estado.
- R2. `SubTotal del mes` = suma de los gastos con estado `OK` (sin distinguir mayúsculas).
- R3. Gastos sin estado no cuentan en el SubTotal.
- R4. `Ingreso del mes` = sueldo + prima.
- R5. `Restante` = ingreso − Total; `SubRestante` = ingreso − SubTotal. Pueden ser negativos.
- R6. Totales anuales = suma de los valores mensuales.
- R7. `Pago de una deuda` = suma de sus cuotas vinculadas + ajuste fijo.
- R8. `Pendiente` = Total − Pago. Puede ser negativo (pago de más).
- R9. Una cuota se suma al pago de la deuda cuando se paga.
- R10. Si un monto es una fórmula aritmética, se guarda el resultado y la fórmula como desglose.

## Particularidades que el importador maneja

- I1. El rango de suma de algunos `SUMIF` es más corto que el de criterio. LibreOffice lo amplía, así
  que el SubTotal equivale a sumar el bloque entero. La app aplica R2 sobre todo el bloque.
- I2. Puede haber gastos fuera de cualquier bloque (entre un `Restante` y el siguiente encabezado).
  Van al mes del encabezado siguiente (o al anterior si no hay), con la marca "fuera de bloque".
- I3. Referencias a celdas vacías valen 0.
- I4. Encabezados con año equivocado: el año sale de la columna, y el log registra la corrección.
- I5. Nombres distintos para la misma deuda: el vínculo usa fórmulas y, para cuotas pendientes, la
  raíz del concepto.
- I6. Deudas con pago fijo sin fórmula: total = suma de sus cuotas (alias en `reglas.txt`).
- I7. Ruido de coma flotante (`…0000003`): se redondea a 2 decimales con registro en el log.
