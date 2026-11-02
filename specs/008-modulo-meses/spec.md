# Spec: Módulo Meses

Pedido del usuario: un módulo con los meses completos y pendientes, un distintivo visual para cada
uno y estadísticas en el Tablero.

## Requisitos

- FR-801: Un mes completo está hecho o todo pagado por confirmar (ver `specs/006-mes-hecho/`). Los
  meses sin gastos no se listan ni cuentan.
- FR-802: El módulo agrupa los meses por confirmar, con pagos pendientes y hechos.
- FR-803: Distintivo con icono y texto en Meses, Resumen, la vista Año y la tabla del Tablero.
- FR-804: Un mes está atrasado si ya pasó y tiene un pago pendiente sin fecha o con fecha pasada.
- FR-805: Cifras: meses completos, por confirmar, con pendientes y atrasados; pagos pendientes;
  falta por pagar y atrasado por pagar. El módulo las calcula sobre todos los meses y el Tablero
  sobre el periodo elegido.
