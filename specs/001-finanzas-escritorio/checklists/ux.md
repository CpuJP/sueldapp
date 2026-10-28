# Checklist: Experiencia de usuario

**Purpose**: Validar que los requisitos de pantallas, flujos y lenguaje visual están completos y
son verificables.
**Created**: 2026-10-28
**Feature**: [spec.md](../spec.md)

## Pantallas y flujos

- [x] CHK001 ¿Cada pantalla mínima tiene requisitos de contenido? (resumen, movimientos, deudas, tarjetas, catálogos, importar/exportar) [FR-013, FR-014, FR-001b, FR-008, FR-011]
- [x] CHK002 ¿Se define qué son "próximos pagos" (gastos no pagados del mes elegido)? [FR-013]
- [x] CHK003 ¿Se define el estado vacío (app sin datos) y su llamada a importar? [US1]
- [x] CHK004 ¿Se define la confirmación y el deshacer de borrados? [FR-010]
- [x] CHK005 ¿Se definen los mensajes de error de validación en lenguaje sencillo? [US3-4]
- [x] CHK006 ¿Se define cómo se distingue un restante negativo sin depender solo del color? [US2-2]

## Lenguaje visual

- [x] CHK007 ¿Están definidas las superficies de vidrio (desenfoque, saturación, borde especular)? [Prompt <diseno>, Research R1]
- [x] CHK008 ¿Están definidos los patrones prohibidos (crema plano, morado-azul, emojis, 01/02/03, monospace decorativo, cuadrícula de tarjetas idénticas)? [Prompt <diseno>]
- [x] CHK009 ¿Se exige alineación tabular de cifras? [FR-012, Prompt <diseno>]
- [x] CHK010 ¿Se define la revisión de capturas en claro, oscuro y reducir transparencia? [Research R7]

## Rendimiento percibido

- [x] CHK011 ¿Hay metas medibles de arranque y respuesta? [SC-003, SC-004, Plan]
- [x] CHK012 ¿Se limitan las transiciones a cortas (≤ 200 ms) y se anulan con reducir movimiento? [FR-015]
