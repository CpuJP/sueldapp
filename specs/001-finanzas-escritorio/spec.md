# Feature Specification: App de escritorio de finanzas personales

**Feature Branch**: `001-finanzas-escritorio`

**Created**: 2026-10-28

**Status**: Draft

**Input**: User description: "Reemplazar la hoja `Ingreso-Egreso.ods` por una app de escritorio local
que importe el 100 % de sus datos, reproduzca su lógica (totales, subtotales, restantes, saldos de
deudas), sea fácil de usar, totalmente editable desde la interfaz y con estilo visual tipo vidrio."
Base: `docs/analisis-hoja.md`.

## Clarifications

### Session 2026-10-28

> Las respuestas concretas (nombres de deudas, tarjetas y montos) son datos personales y viven
> fuera del repositorio (`docs/privado/`, `reglas.txt`). Aquí queda la regla general.

- Q: ¿Qué hacer con los gastos que están fuera de cualquier bloque mensual? → A: Se asignan a un mes
  con la marca "fuera de bloque" y no cuentan en el total del mes (igual que la hoja). El usuario
  puede quitar la marca después.
- Q: ¿Cómo modelar una deuda cuyo pago en la hoja es un valor fijo sin fórmula? → A: Como crédito a
  cuotas: Total = suma de sus cuotas (vinculadas por nombre), Pagado = cuotas pagadas. El valor fijo
  de la hoja no se reproduce: la conciliación lo registra como diferencia aceptada por el usuario.
- Q: ¿Cómo importar pagos de tarjetas de crédito, préstamos de terceros y compras sueltas? → A: Las
  tarjetas de crédito van a una sección propia "Tarjetas" con sus pagos mensuales (pago mínimo)
  vinculados; los préstamos de terceros son gastos con categoría "Préstamos"; las compras sueltas son
  gastos normales. Qué palabra del concepto corresponde a qué tarjeta se configura en `reglas.txt`.
- Q: ¿Qué hacer con pestañas que no son la de nómina (p. ej. cotizaciones)? → A: No se importan; el
  log las registra como omitidas por decisión del usuario.
- Q: ¿Qué datos lleva cada tarjeta de crédito? → A: Nombre, cupo, saldo actual (editado a mano),
  día de corte y día de pago, más sus pagos mensuales vinculados.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Importar la hoja sin perder nada (Priority: P1)

Abro la app por primera vez, elijo mi archivo `Ingreso-Egreso.ods` y la app trae todos mis meses,
gastos, sueldos, primas y deudas. Al terminar me dice cuántas filas importó, cuáles transformó y
cuáles rechazó, y si los totales coinciden con la hoja.

**Why this priority**: sin mis datos la app no sirve; es el reemplazo de la hoja.

**Independent Test**: importar el `.ods` real y comprobar que la conciliación da cero diferencias.

**Acceptance Scenarios**:

1. **Given** una app vacía, **When** importo el `.ods`, **Then** aparecen todos los meses,
   todos los gastos con su estado y todas las deudas.
2. **Given** la importación terminada, **When** reviso el informe, **Then** veo la conciliación
   por mes, por año y por deuda, con cero diferencias.
3. **Given** que ya importé, **When** importo el mismo archivo otra vez, **Then** no se duplica
   ningún dato.
4. **Given** cualquier importación, **Then** el archivo original no cambia (misma huella antes y
   después).

---

### User Story 2 - Ver el resumen de un mes (Priority: P1)

Abro la app y veo el mes actual: ingresos (sueldo + prima + otros), total de gastos, lo ya pagado,
lo que me queda (restante y restante de lo pagado) y los pagos pendientes del mes. Puedo moverme a
otros meses.

**Why this priority**: es lo que hoy miro en la hoja cada mes.

**Independent Test**: abrir un mes importado y comparar Total, SubTotal, Sueldo, Restante y
SubRestante con los de la hoja.

**Acceptance Scenarios**:

1. **Given** datos importados, **When** abro un mes, **Then** los cinco valores coinciden con la hoja.
2. **Given** un mes con restante negativo, **Then** el valor se distingue de forma clara (no solo por
   color).
3. **Given** el resumen, **When** elijo un año, **Then** veo total ingresos, total gastos y total
   restante del año.

---

### User Story 3 - Registrar y editar movimientos (Priority: P1)

Añado un gasto o un ingreso a un mes, cambio su monto, su concepto o su estado (por ejemplo de
"Pendiente" a "Pagado"), lo borro, o lo vinculo a una deuda como cuota. Busco y filtro movimientos
por texto, mes, estado, categoría o deuda.

**Why this priority**: sustituye la edición de celdas.

**Independent Test**: marcar como pagado un gasto pendiente y ver el SubTotal del mes y el saldo de
la deuda actualizados al instante.

**Acceptance Scenarios**:

1. **Given** un gasto pendiente vinculado a una deuda, **When** lo marco pagado, **Then** sube el
   pagado del mes y baja el pendiente de la deuda.
2. **Given** un movimiento, **When** lo borro, **Then** la app pide confirmación y permite deshacer
   en ese momento.
3. **Given** el listado, **When** escribo "tarjeta", **Then** solo veo movimientos cuyo concepto lo
   contiene, sin importar mayúsculas ni tildes.
4. **Given** un monto inválido (texto, vacío), **Then** la app no guarda y explica el error en
   lenguaje sencillo.

---

### User Story 4 - Gestionar deudas (Priority: P2)

Veo cada deuda con total, pagado, pendiente, progreso, número de cuotas pagadas y fecha estimada de
fin. Creo, edito y borro deudas, y marco una como saldada.

**Independent Test**: cada deuda muestra el mismo pagado y pendiente que la hoja.

**Acceptance Scenarios**:

1. **Given** datos importados, **Then** el pagado y pendiente de cada deuda coinciden con la hoja.
2. **Given** una deuda con cuotas futuras pendientes, **Then** la fecha de fin es el mes de la
   última cuota registrada.
3. **Given** una deuda pagada de más, **Then** el pendiente negativo se muestra como "pagado de más".

---

### User Story 4b - Tarjetas de crédito (Priority: P2)

Veo mis tarjetas de crédito con cupo, saldo actual, cupo disponible, día de corte y de pago, y
la lista de pagos mensuales vinculados con su total. Creo, edito y borro tarjetas.

**Independent Test**: cada tarjeta muestra todos sus pagos de la hoja y su suma.

**Acceptance Scenarios**:

1. **Given** datos importados, **Then** las tarjetas existen con sus pagos vinculados; cupo, saldo y
   días quedan vacíos hasta que los edite.
2. **Given** cupo y saldo, **Then** cupo disponible = cupo − saldo.
3. **Given** un pago de tarjeta pendiente en el mes actual, **Then** aparece en "próximos pagos".

---

### User Story 5 - Catálogos editables (Priority: P2)

Edito los catálogos que hoy están fijos en la hoja: estados (Pagado, Pendiente, No, Aplazado),
categorías y fuentes de ingreso (Sueldo, Prima, Otro). Puedo crear, renombrar y borrar elementos.

**Acceptance Scenarios**:

1. **Given** un estado en uso, **When** intento borrarlo, **Then** la app me pide elegir a qué estado
   pasan sus movimientos.
2. **Given** un estado, **Then** puedo indicar si cuenta como "pagado" para los subtotales.

---

### User Story 6 - Exportar y volver a importar (Priority: P2)

Exporto todo a `.ods` y a `.csv` para no quedar atrapado. Puedo volver a importar un `.ods`.

**Acceptance Scenarios**:

1. **Given** datos en la app, **When** exporto a `.ods` y lo vuelvo a importar en una app vacía,
   **Then** obtengo los mismos totales.
2. **Given** una exportación `.csv`, **Then** se abre en cualquier hoja de cálculo con montos
   exactos.

---

### User Story 7 - Apariencia y accesibilidad (Priority: P3)

La app sigue el modo claro/oscuro del sistema, tiene superficies tipo vidrio translúcido, una
opción "reducir transparencia" y respeta "reducir movimiento". Todo se puede usar con teclado.

**Acceptance Scenarios**:

1. **Given** "reducir transparencia" activado, **Then** todas las superficies son sólidas.
2. **Given** solo teclado, **Then** puedo llegar a cualquier pantalla y acción, con foco visible.

### Edge Cases

- `.ods` sin la estructura esperada o dañado: la importación se cancela sin escribir nada y explica
  el motivo.
- Montos en texto, vacíos o con ruido de coma flotante: se rechazan o redondean, registrando el
  motivo en el log.
- Gastos fuera de cualquier bloque mensual: se importan con una marca visible (ver
  Clarifications) y la conciliación reproduce los totales actuales.
- Fórmulas aritméticas en montos (`=49900*2`): se guarda el resultado y la fórmula como desglose.
- Encabezados de mes con año equivocado (p. ej. "Octubre 2026" en la columna de 2027): el año sale de la
  columna, y el log registra la corrección.
- Restantes negativos y deudas con pendiente negativo.
- Borrar una deuda con cuotas vinculadas: las cuotas se conservan como gastos sin deuda, tras
  confirmación.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: El sistema MUST importar todas las celdas con datos de la pestaña `Nómina` y asociar
  cada registro a su celda de origen. Las demás pestañas se omiten y quedan registradas en el log.
- **FR-001b**: El sistema MUST gestionar tarjetas de crédito (nombre, cupo, saldo actual, día de
  corte, día de pago) con sus pagos mensuales vinculados; cupo disponible = cupo − saldo.
- **FR-001c**: La conciliación MUST aceptar solo las diferencias aprobadas por el usuario en
  Clarifications (deudas sin fórmula de pago) y listarlas por separado.
- **FR-002**: El sistema MUST NOT modificar el `.ods` de origen.
- **FR-003**: La importación MUST ser idempotente: importar dos veces el mismo archivo no duplica.
- **FR-004**: La importación MUST escribir un log con cada fila importada, transformada o rechazada
  y su motivo.
- **FR-005**: El sistema MUST ejecutar una conciliación tras importar: Total, SubTotal, Sueldo,
  Restante y SubRestante por mes; totales por año; Pago, Total y Pendiente por deuda. Cualquier
  diferencia marca la importación como fallida y se muestra.
- **FR-006**: Total del mes = suma de todos sus gastos. Pagado del mes (SubTotal) = suma de gastos
  cuyo estado cuenta como pagado. Ingreso del mes = suma de sus ingresos. Restante = Ingreso − Total.
  Restante de lo pagado = Ingreso − Pagado.
- **FR-007**: Pagado de una deuda = suma de cuotas vinculadas con estado pagado + ajuste manual.
  Pendiente = Total − Pagado.
- **FR-008**: Los usuarios MUST poder crear, editar y borrar meses, gastos, ingresos, deudas, estados,
  categorías y fuentes de ingreso.
- **FR-009**: El listado de movimientos MUST permitir buscar por texto (sin distinguir mayúsculas ni
  tildes) y filtrar por mes, año, estado, categoría y deuda.
- **FR-010**: Todo borrado MUST pedir confirmación y ofrecer deshacer inmediato.
- **FR-011**: El sistema MUST exportar todos los datos a `.ods` y `.csv` e importar de nuevo un `.ods`
  propio o con el formato de la hoja original.
- **FR-012**: Los montos MUST ser exactos a 2 decimales en todo cálculo, almacenamiento y
  exportación. Formato de pantalla: `$ 1.234.567,89` (COP).
- **FR-013**: La pantalla de resumen MUST mostrar ingresos, gastos, pagado, restante y próximos
  pagos pendientes del mes elegido.
- **FR-014**: La pantalla de deudas MUST mostrar total, pagado, pendiente, % de progreso, cuotas
  pagadas/registradas y mes estimado de fin.
- **FR-015**: La app MUST seguir el modo claro/oscuro del sistema, ofrecer "reducir transparencia" y
  respetar "reducir movimiento". Las transiciones duran ≤ 200 ms y desaparecen con reducir movimiento.
- **FR-016**: Toda la app MUST poderse usar con teclado: orden de tabulación lógico, foco visible,
  diálogos que retienen el foco y se cierran con Escape, etiquetas accesibles en todos los campos y
  botones. Los estados se muestran con texto, no solo con color.
- **FR-019**: Sin datos, la app MUST mostrar un estado vacío que invita a importar el `.ods`.
- **FR-020**: "Próximos pagos" = gastos del mes elegido cuyo estado no cuenta como pagado.
- **FR-017**: La app MUST NOT hacer conexiones de red.
- **FR-018**: Los datos MUST guardarse de forma local y persistente; un cierre inesperado no pierde
  cambios ya confirmados.

### Key Entities

- **Mes (periodo)**: año, mes, marca "cerrado". Tiene ingresos y gastos.
- **Ingreso**: mes, fuente (Sueldo, Prima, Otro…), monto, desglose opcional (fórmula original).
- **Gasto (movimiento)**: mes, concepto, monto, estado, categoría opcional, deuda opcional,
  tarjeta opcional, desglose
  opcional, nota, celda de origen, marca "fuera de bloque".
- **Estado**: nombre, ¿cuenta como pagado?, color. Iniciales: Pagado (OK), Pendiente, No, Aplazado,
  Sin estado.
- **Deuda**: nombre, monto total, ajuste manual de pagado, marca "saldada", notas. Sus cuotas son
  gastos vinculados.
- **Tarjeta de crédito**: nombre, cupo, saldo actual, día de corte, día de pago. Sus pagos son
  gastos vinculados.
- **Categoría**: nombre. Catálogo editable. Iniciales: Tarjetas, Préstamos, Servicios, Personal.
- **Fuente de ingreso**: nombre. Catálogo editable.
- **Importación**: fecha, huella del archivo, resultado de la conciliación, log.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: La conciliación contra el `.ods` original da 0 diferencias en todos los meses, años y
  todas las deudas; las únicas diferencias aceptadas son las de deudas sin fórmula de pago y aparecen
  listadas como tales.
- **SC-002**: Importar el mismo archivo 2 veces deja exactamente el mismo número de registros.
- **SC-003**: Marcar un gasto como pagado y ver el resumen actualizado toma menos de 3 acciones y
  menos de 1 segundo.
- **SC-004**: La app abre y muestra el resumen en menos de 2 segundos con los datos actuales.
- **SC-005**: Todo el texto cumple contraste AA en modo claro, oscuro y "reducir transparencia".
- **SC-006**: El 100 % de las acciones se puede hacer solo con teclado.
- **SC-007**: Exportar y reimportar conserva el 100 % de los totales.

## Assumptions

- Un solo usuario, en un solo equipo Linux (Fedora). Sin cuentas ni sincronización.
- Moneda única COP; no hay movimientos en otra moneda.
- Los meses 2027–2028 se importan tal cual (proyección con estado Pendiente/sin estado).
- Los colores de fondo de la hoja solo reflejan el estado; no se importan como dato aparte.
- El estado `Ok` se trata igual que `OK`.
- Los errores de escritura de la hoja se importan tal cual (el usuario puede editarlos luego), salvo
  que la clarificación diga otra cosa.

### Out of scope

- Presupuestos automáticos, IA, gráficos de predicción, sincronización, multiusuario, multimoneda,
  conexión con bancos, app móvil.
