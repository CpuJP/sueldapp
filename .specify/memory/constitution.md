<!--
Sync Impact Report
- Version change: template → 1.0.0
- Principles added: I. Datos locales y privados; II. Migración sin pérdida verificable;
  III. Dominio probado antes que UI; IV. Dinero exacto; V. Accesibilidad;
  VI. Simplicidad; VII. Puertas de calidad
- Sections added: Restricciones técnicas; Flujo de trabajo
- Removed sections: none
- Deferred TODOs: none
-->
# Sueldapp Constitution

## Core Principles

### I. Datos locales y privados (NO NEGOCIABLE)
- Todos los datos financieros MUST vivir solo en el equipo del usuario (SQLite local).
- La app MUST NOT hacer llamadas de red en tiempo de ejecución: sin telemetría, analítica,
  actualizaciones automáticas, fuentes web remotas ni CDNs.
- Los archivos con datos (`.ods`, base de datos, logs de importación, exportaciones) MUST estar
  excluidos de git.

Razón: son las finanzas personales del usuario; nada justifica que salgan del equipo.

### II. Migración sin pérdida verificable (NO NEGOCIABLE)
- El archivo `.ods` original MUST NOT modificarse jamás; el importador solo lo lee.
- El importador MUST ser idempotente y registrar en un log cada fila importada, transformada o
  rechazada, con su motivo y su celda de origen.
- Una prueba automática de conciliación MUST comparar totales por mes, por año, por estado y el
  saldo de cada deuda contra los valores de la hoja. Cualquier diferencia = migración fallida.
- El usuario MUST poder exportar todos sus datos a `.ods` y `.csv`.

### III. Dominio probado antes que UI
- Toda regla de negocio (totales, subtotales, restantes, saldos de deuda) MUST vivir en un crate
  de dominio en Rust, sin dependencias de UI ni de base de datos.
- Cada regla MUST tener pruebas antes de exponerse en la interfaz.
- La UI MUST NOT calcular dinero; solo muestra lo que devuelve el dominio.

### IV. Dinero exacto
- Los montos MUST usar `rust_decimal::Decimal`. `f64`/`f32` MUST NOT usarse para dinero en
  ninguna capa (en la base de datos se guardan como texto decimal o entero en centavos).
- Valores de la hoja con ruido de coma flotante se redondean a 2 decimales al importar y el
  redondeo queda registrado en el log.

### V. Accesibilidad
- Contraste de texto WCAG AA en modo claro, oscuro y "reducir transparencia".
- Navegación completa con teclado y foco visible.
- Respeto a "reducir movimiento" del sistema y opción propia de "reducir transparencia".
- Lenguaje sencillo, en español, sin jerga técnica.

### VI. Simplicidad
- Solo se construye lo que pide la especificación aprobada. Ideas extra van a `docs/ideas.md`.
- Sin abstracciones "para más adelante". Un usuario, un equipo, sin cuentas ni sincronización.
- Cada dependencia nueva MUST justificarse en el plan o en un ADR.

### VII. Puertas de calidad
- Ninguna tarea se cierra sin `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
  y `cargo test` en verde (incluida la conciliación).

## Restricciones técnicas

- Lenguaje del núcleo: Rust estable. Persistencia: SQLite con migraciones versionadas.
- Moneda única: COP.
- Sistema objetivo principal: Linux (Fedora); el código no debe atarse a una sola plataforma sin
  necesidad.

## Flujo de trabajo

- Spec Kit: constitución → spec → clarify → plan → checklist → tasks → analyze → implement.
- `tasks.md` es la fuente de verdad del progreso; cada tarea se marca al terminarla.
- Decisiones importantes se registran como ADR en `docs/decisiones/`.

## Governance

- Esta constitución prevalece sobre cualquier otra práctica del proyecto.
- Enmiendas: se documentan en este archivo con versión semántica (MAJOR al quitar o redefinir un
  principio, MINOR al añadir, PATCH al aclarar) y requieren aprobación del usuario.
- Cada plan y revisión de código MUST verificar el cumplimiento; el subagente `revisor` lo
  comprueba. Guía operativa: `AGENTS.md`.

**Version**: 1.0.0 | **Ratified**: 2026-10-28 | **Last Amended**: 2026-10-28
