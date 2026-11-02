# Spec: Fecha de pago de las cuotas

Pedido del usuario: anotar en cada cuota de una deuda la fecha en que se paga (por ejemplo, la
cuota 1 el 23 de noviembre).

## Requisitos

- FR-501: Un gasto (y por tanto una cuota de deuda) puede tener una fecha de pago opcional.
- FR-502: Se escribe como día/mes/año; una fecha inexistente muestra un error en lenguaje sencillo
  y no guarda.
- FR-503: La fecha se muestra en la fila del gasto, también en las cuotas de la deuda.
- FR-504: La fecha sobrevive a exportar y restaurar; las exportaciones anteriores siguen
  restaurándose, sin fecha.
- FR-505: La fecha no cambia ningún total ni el mes al que pertenece el gasto.

## Fuera de alcance

Generar varias cuotas de una vez: ver `docs/ideas.md`. Las alertas están en `specs/007-alertas-de-pago/`.
