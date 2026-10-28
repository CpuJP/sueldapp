# Migración desde `Ingreso-Egreso.ods`

## Cómo importar

- **Desde la app:** Importar y exportar → *Elegir archivo e importar*.
- **Desde la terminal:**
  ```bash
  cargo run --release -p sueldapp-ods --bin importar -- Ingreso-Egreso.ods
  ```
  El log completo queda en `logs/importacion-<fecha>.log`.

El archivo solo se lee; nunca se modifica.

Toda la importación ocurre en una transacción. Si algo falla o la conciliación encuentra una
diferencia, no se guarda nada y la importación queda registrada como fallida.

## Qué se importa

| Hoja | Destino |
|---|---|
| `Nómina` A8:E19 (deudas) | Deudas. El total viene de la columna C. Las cuotas se vinculan según las referencias de la fórmula de la columna B; las constantes de esa fórmula pasan a "pagado sin cuota" (ajuste). |
| `Nómina`, bloques `Sueldo <Mes> <Año>` | Un mes por bloque. La celda `OK` junto al encabezado marca el mes como cerrado. |
| Filas de concepto/monto/estado | Gastos. Estado: `OK`/`Ok` → Pagado, `PENDIENTE` → Pendiente, `NO` → No, `APLAZADO` → Aplazado, vacío → Sin estado. Si el monto era una fórmula, se guarda como "desglose". |
| `Sueldo` y `Prima` | Ingresos. Cuando la fórmula del sueldo suma la prima, la prima se separa y el sueldo queda sin ella. |
| Otras pestañas | No se importan (decisión del usuario). Cada fila queda en el log como omitida. |

Reglas acordadas con el usuario (ver `specs/001-finanzas-escritorio/spec.md`, Clarifications):

- Los gastos que estaban fuera de cualquier mes pasan a un mes con la marca "no cuenta en el
  total", igual que en la hoja.
- Una deuda con pago fijo sin fórmula se trata como crédito a cuotas: su total es la suma de sus
  cuotas y su pagado, la de las cuotas pagadas. Es la única diferencia aceptada frente a la hoja.
- Las cuotas *pendientes* que no aparecen en ninguna fórmula se vinculan por nombre a su deuda. Por
  ejemplo, "Consola XL – Cuota 6" se vincula a "Consola XL". Las cuotas pagadas nunca se vinculan
  por nombre, así que ningún total cambia.
- Si un año escrito en un encabezado no coincide con su columna, se usa el año de la columna, y el
  log lo registra.
- Los valores con ruido de coma flotante se redondean a 2 decimales, y el log registra cada caso.

### Reglas personales (`reglas.txt`)

Las reglas que dependen de tus datos (palabras que identifican cada tarjeta, alias de deudas,
categorías con nombres propios) van en `reglas.txt`, junto a la base de datos
(`~/.local/share/sueldapp/reglas.txt` en Linux, `%APPDATA%\sueldapp\reglas.txt` en Windows) o en
la ruta de `SUELDAPP_REGLAS`. Ese archivo **no se versiona**. Ejemplo: `docs/ejemplos/reglas.ejemplo.txt`.

```text
tarjeta: banco azul = Tarjeta Azul        # palabra del concepto = tarjeta
alias: moto nueva = moto                  # deuda sin fórmula = raíz de sus cuotas
categoria: cel ana = Tecnología           # palabras del concepto = categoría
```

Sin `reglas.txt` se usan solo las reglas genéricas del código (servicios, hogar, alimentación…).
Para categorizar una base ya importada sin tocar lo que ya tiene categoría:
`sueldapp-importar --categorizar`.

## Idempotencia

- Si importas el mismo archivo (misma huella SHA-256) por segunda vez, no cambia nada.
- Con "volver a importar", cada registro se actualiza por su celda de origen. No se crean
  duplicados, y lo que creaste en la app se conserva.
- El cupo, el saldo y los días de las tarjetas no se pisan al reimportar.

## Cómo verificar

```bash
cargo test -p sueldapp-ods --test conciliacion
```

La prueba importa el `.ods` real en una base en memoria y comprueba lo siguiente:

- 0 diferencias en todos los valores: Total, SubTotal, Sueldo, Restante y SubRestante de cada mes;
  TOTAL PAGO/GASTO/RESTANTE de cada año; Pago, Total y Pendiente de cada deuda.
- Las diferencias aceptadas son solo las de deudas sin fórmula de pago.
- Al importar dos veces, los conteos no cambian.
- Al exportar a `.ods` y restaurar en una base vacía, los conteos son iguales.
- La huella del archivo original no cambia.


## Exportar y volver

- **`.ods`:** una hoja por tabla, con montos exactos y columnas legibles (mes, estado, deuda…).
  Si importas este archivo, se *reemplazan* todos los datos, y la hoja `Resumen` sirve para
  verificar los totales.
- **`.csv`:** un archivo por tabla, en UTF-8, con punto decimal.
