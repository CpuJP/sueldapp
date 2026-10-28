# Reglas de datos financieros

- Montos: `rust_decimal::Decimal`; `TEXT` en SQLite; string en JSON. Nunca `f64`.
- Redondeo a 2 decimales, punto medio lejos de cero, solo al importar valores con ruido.
- El `.ods` original es de solo lectura.
- Importación idempotente por celda de origen; toda fila con línea de log.
- Sin red: ninguna dependencia ni recurso que conecte en tiempo de ejecución.
- Datos reales fuera de git y de `docs/`.
