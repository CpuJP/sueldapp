# Spec: Alertas de pagos cercanos

Pedido del usuario: un sistema de alertas y notificaciones de pagos cercanos.

## Requisitos

- FR-701: Hay alerta para un gasto que espera pago (ver `specs/006-mes-hecho/`), cuenta en los
  totales y tiene fecha de pago vencida o a 7 días o menos, según la fecha local del equipo.
- FR-702: El menú lateral muestra cuántos hay mientras existan; abre la lista, del más urgente al
  menos urgente, con opciones para marcar como pagado y editar.
- FR-703: Una notificación avisa cuando aparece una alerta nueva, también al abrir la app. Cada pago
  avisa una vez como próximo y otra al vencer.
- FR-704: La lista se actualiza con cada cambio y al menos cada hora con la app abierta.

## Fuera de alcance

Notificaciones del sistema, avisos con la app cerrada y el día de pago de las tarjetas: ver
`docs/ideas.md`.
