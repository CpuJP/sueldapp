# Data Model

Montos: `Decimal` en Rust, `TEXT` en SQLite. `origen` = celda del `.ods` (`Nómina!L163`), único y
nulo para registros creados en la app.

## meses
| Campo | Tipo | Regla |
|---|---|---|
| id | INTEGER PK | |
| anio | INTEGER | 2000–2100 |
| mes | INTEGER | 1–12; único (anio, mes) |
| cerrado | BOOL | celda `OK` junto al encabezado |

## estados
| id | nombre (único) | cuenta_pagado | orden |
Iniciales: Pagado (sí), Pendiente, No, Aplazado, Sin estado (no). Borrar exige reasignar.

## categorias / fuentes_ingreso
| id | nombre (único) |
Categorías iniciales: Tarjetas, Préstamos, Servicios, Personal. Fuentes: Sueldo, Prima, Otro.

## deudas
| Campo | Tipo | Regla |
|---|---|---|
| id | PK | |
| nombre | TEXT | no vacío |
| total | TEXT? | ≥ 0; nulo = sin total |
| ajuste | TEXT | default 0; suma al pagado |
| saldada | BOOL | marca manual |
| notas | TEXT | |
| origen | TEXT? único | |

Derivados (dominio): pagado = Σ cuotas con estado pagado + ajuste; pendiente = total − pagado;
progreso = pagado / total (0–100 %, tope 100); cuotas pagadas/registradas; fin estimado = mes de la
última cuota vinculada.

## tarjetas
| id | nombre | cupo? | saldo? | dia_corte? (1–31) | dia_pago? (1–31) | origen? |
Derivados: disponible = cupo − saldo; total pagado = Σ pagos pagados.

## gastos
| Campo | Tipo | Regla |
|---|---|---|
| id | PK | |
| mes_id | FK meses | obligatorio, ON DELETE CASCADE |
| concepto | TEXT | no vacío |
| monto | TEXT | ≥ 0 |
| estado_id | FK estados | obligatorio |
| categoria_id | FK? | ON DELETE SET NULL |
| deuda_id | FK? | ON DELETE SET NULL |
| tarjeta_id | FK? | ON DELETE SET NULL |
| desglose | TEXT? | fórmula original (`=49900*2`) |
| nota | TEXT | |
| fuera_de_bloque | BOOL | excluido de totales |
| orden | INTEGER | orden dentro del mes |
| origen | TEXT? único | |

## ingresos
| id | mes_id FK | fuente_id FK | monto | desglose? | nota | origen? único |

## importaciones
| id | fecha | archivo | huella SHA-256 | ok BOOL | resumen TEXT | log TEXT |

## Reglas del mes (dominio)
- Total = Σ montos de gastos con `fuera_de_bloque = false`.
- Pagado (SubTotal) = Σ de los anteriores con estado `cuenta_pagado`.
- Ingreso = Σ ingresos.
- Restante = Ingreso − Total; Restante de lo pagado = Ingreso − Pagado.
- Año: Σ de cada valor mensual.

## Transiciones
- Gasto: cualquier estado → cualquier estado (edición libre).
- Deuda: `saldada` manual; la UI sugiere marcarla cuando pendiente ≤ 0.
