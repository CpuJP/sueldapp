# Feature Specification: Plantilla para llenar a mano y publicación del repositorio

**Feature Branch**: `003-plantilla-y-publicacion`

**Created**: 2026-10-28

**Status**: Implementado

## User Scenarios & Testing

### US1 — Plantilla (P1)

Descargo desde la app un `.ods` con las hojas Gastos, Ingresos, Deudas y Tarjetas, instrucciones
y una hoja con los nombres válidos. Lo lleno en cualquier hoja de cálculo y lo importo.

**Acceptance**:

1. Importar una plantilla llena suma sus registros a los existentes, sin borrar nada.
2. Importar el mismo contenido otra vez no duplica nada (origen por contenido de la fila).
3. Si una fila tiene un error (año, mes, monto, estado), no se guarda nada y el informe dice la fila.
4. Categorías, deudas, tarjetas y fuentes desconocidas se crean por nombre.

### US2 — Repositorio público (P1)

El código se publica en GitHub sin datos personales, con licencia MIT, documentación, plantillas
de issues y PR, CI (Linux y Windows, x64 y ARM64) y releases automáticas con instaladores.

**Acceptance**:

1. Ningún archivo versionado ni commit contiene montos o nombres reales.
2. Las reglas personales viven en `reglas.txt` (no versionado); el código solo trae reglas genéricas.
3. Una etiqueta `vX.Y.Z[-pre]` genera un borrador de release con instaladores y `SHA256SUMS.txt`.

## Requirements

- **FR-301**: `plantilla::tablas_plantilla` y `plantilla::importar`, con pruebas.
- **FR-302**: Ruta de datos por sistema: Linux XDG, Windows `%APPDATA%`, macOS Application Support.
- **FR-303**: `Reglas` = genéricas + `reglas.txt`; la conciliación real usa las reglas personales.
