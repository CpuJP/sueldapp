# Quickstart

```bash
cargo test --workspace                 # dominio, almacén, importador y conciliación
cargo run -p sueldapp                  # abre la app
cargo run -p sueldapp-ods --bin importar -- Ingreso-Egreso.ods   # importar desde terminal
```

Validación manual:
1. Abrir la app vacía → Importar/Exportar → elegir `Ingreso-Egreso.ods`.
2. Informe: 0 diferencias (las deudas sin fórmula de pago aparecen como diferencias aceptadas).
3. Resumen → un mes cualquiera: Total, Pagado y Restante iguales a los de la hoja.
4. Movimientos → una cuota pendiente → marcar Pagado → el pendiente de su deuda baja ese monto.
5. Importar el mismo archivo otra vez → mismo número de registros.
