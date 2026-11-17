# Spec: Número de cuotas de una deuda

Pedido del usuario: al ingresar una deuda, poder poner el número de cuotas si se conoce.

## Requisitos

- FR-1101: Una deuda tiene un número de cuotas opcional, entero y de 1 o más.
- FR-1102: Con número de cuotas, "Cuotas pagadas" muestra "pagadas de N" y el fin estimado es el mes
  de la primera cuota registrada más N − 1, o el de la última registrada si es posterior.
- FR-1103: Sin número, todo funciona como antes.
- FR-1104: Se conserva al exportar, restaurar y reimportar la hoja; las exportaciones anteriores
  se restauran sin número.
