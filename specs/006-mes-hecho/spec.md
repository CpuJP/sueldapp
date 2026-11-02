# Spec: Mes hecho

Pedido del usuario: un mes queda OK solo si ya se pagó todo. Si la app detecta que se pagó todo,
pregunta si se marca como hecho; la confirmación es manual y los meses anteriores se pueden editar.

## Reglas

- Pago pendiente: gasto del mes que cuenta en los totales, sin estado pagado y con un estado
  distinto de "No" y "Aplazado", que son decisiones ya tomadas.
- FR-601: Situación del mes: sin gastos, faltan N pagos, todo pagado (por confirmar) o hecho.
- FR-602: Un mes con pagos pendientes no se puede marcar como hecho.
- FR-603: Un mes hecho que vuelve a tener un pago pendiente deja de estar hecho y se avisa.
- FR-604: Cuando un mes pasa a no tener pagos pendientes y no está hecho, se pregunta. Nunca se
  marca solo. Si varios cambian a la vez (importar, restaurar) no se pregunta por cada uno.
- FR-605: "Ahora no" deja el mes por confirmar, con un botón en Resumen y sin volver a preguntar
  hasta que vuelva a tener pendientes.
- FR-606: Cualquier mes se puede marcar o desmarcar desde Editar mes, y la vista Año muestra la
  situación de cada uno. Marcarlo se puede deshacer.
