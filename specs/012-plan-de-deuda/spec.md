# Spec: Plan de pagos de una deuda

Pedido del usuario: al crear una deuda, indicar el mes de la primera cuota para crear el resto de
cuotas, y un interés mensual o efectivo anual para calcular el pago final.

## Requisitos

- FR-1201: La deuda guarda el mes de la primera cuota y el interés en porcentaje, mensual o efectivo
  anual (mensual = (1 + EA)^(1/12) − 1). Ambos son opcionales.
- FR-1202: Con total, número de cuotas e interés, el plan es de cuota fija (sistema francés); cada
  cuota se redondea a centavos y la última salda el resto. Sin interés: partes iguales.
- FR-1203: El formulario muestra la cuota, el total a pagar y los intereses mientras se escribe.
- FR-1204: Al guardar, si se pide, se crea una cuota pendiente por cada mes del plan que no tenga ya
  una cuota de esa deuda, con el monto del plan (o el de la cuota registrada si no hay total) y la
  categoría de la cuota registrada. Todo en una transacción, con opción de deshacer.
- FR-1205: Con interés, el pagado, lo pendiente y el progreso se miden contra el total a pagar.
- FR-1206: Se conserva al exportar, restaurar y reimportar.
