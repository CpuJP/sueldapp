# Checklist: Migración de datos

**Purpose**: Validar que los requisitos de importación, conciliación y exportación están completos,
claros y son medibles.
**Created**: 2026-10-28
**Feature**: [spec.md](../spec.md)

## Completitud

- [x] CHK001 ¿Está definido qué pestañas se importan y cuáles se omiten, con motivo? [Spec FR-001, Clarifications]
- [x] CHK002 ¿Está definido el destino de cada zona de `Nómina` (deudas, bloques mensuales, totales anuales)? [Data Model, Research R5]
- [x] CHK003 ¿Se especifica qué pasa con las filas fuera de bloque? [Clarifications]
- [x] CHK004 ¿Se especifica cómo se vinculan cuotas a deudas y tarjetas? [Research R5]
- [x] CHK005 ¿Se especifica el tratamiento de fórmulas aritméticas en montos (desglose)? [Edge Cases]
- [x] CHK006 ¿Se especifica qué contiene cada línea del log (fila, celda, acción, motivo)? [FR-004]

## Claridad y medibilidad

- [x] CHK007 ¿Se listan los valores exactos que compara la conciliación? [FR-005]
- [x] CHK008 ¿Está definida la tolerancia de comparación (2 decimales, sin margen)? [Research R6]
- [x] CHK009 ¿Están explícitas las únicas diferencias aceptadas (deudas sin fórmula) y cómo se reportan? [FR-001c, SC-001]
- [x] CHK010 ¿"Idempotente" está definido con una clave verificable (celda de origen)? [Research R5]

## Casos límite

- [x] CHK011 ¿Se define el comportamiento ante `.ods` dañado o con otra estructura (sin escritura parcial)? [Edge Cases]
- [x] CHK012 ¿Se define qué pasa al reimportar tras editar datos en la app? [Research R5 — upsert por origen]
- [x] CHK013 ¿Se define el redondeo del ruido de coma flotante y su registro en el log? [Constitución IV]

## Reversibilidad

- [x] CHK014 ¿Se exige exportar a `.ods` y `.csv` con montos exactos? [FR-011, FR-012]
- [x] CHK015 ¿Se exige que exportar + reimportar conserve los totales? [SC-007]
- [x] CHK016 ¿Se garantiza que el `.ods` original no cambia (huella antes/después)? [FR-002, US1-4]
