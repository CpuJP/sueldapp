# Changelog

Todos los cambios notables de este proyecto se documentan aquí.
El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el proyecto usa
[Versionado Semántico](https://semver.org/lang/es/).

## [Sin publicar]

## [0.3.0] - 2026-10-28

### Añadido

- Al pasar el puntero sobre una barra o un mes de los gráficos se muestra qué contiene (antes solo
  aparecía al hacer clic y arrastrar).
- Notificaciones nuevas: píldora con franja de color y ícono según el tipo (verde: hecho;
  naranja: borrado, deshecho o atención; rojo: error), botón para cerrar y «Deshacer» bien alineado.

### Documentación

- Revisión completa tras el paso a Slint: nombres reales de los instaladores, política de
  seguridad sin CSP ni WebView2, guía de usuario (primer uso, avisos, tooltips), comando de capturas
  y notas en las specs históricas.

## [0.2.0] - 2026-10-28

Primera versión estable.

### Cambiado

- La interfaz ahora es nativa (Slint) en lugar de WebKit: mismo diseño, cuatro a cinco veces menos
  memoria (unos 40–50 MiB en lugar de ~210) y un solo proceso. Ver la ADR 0006.
- Los instaladores ya no dependen de WebKitGTK.
- Ajustes muestra la atribución «Hecho con Slint».

## [0.1.0-alpha.2] - 2026-10-28

### Cambiado

- Nuevo ícono de la app (tablero con moneda, estilo vidrio) en todos los tamaños y en la barra lateral.
- La plantilla para llenar a mano ya no incluye datos de la base (nombres de deudas, tarjetas,
  categorías o fuentes propias): es igual para todos y se puede compartir.

## [0.1.0-alpha.1] - 2026-10-28

Primera versión alpha.

### Añadido

- Importador de la hoja `Ingreso-Egreso.ods` con log por fila, idempotencia y conciliación exacta
  (si algo no cuadra no se guarda nada).
- Plantilla `.ods` para llenar a mano y cargar después; suma datos sin borrar nada.
- Exportación a `.ods` (restaurable) y `.csv`.
- Tablero de estadísticas con gráficos (ingresos y gastos, sobrante o faltante, saldo acumulado,
  desgloses por destino, categoría, estado, concepto y fuente) y selector de periodo con calendario.
- Resumen mensual y anual, movimientos con filtros y búsqueda, deudas a cuotas, tarjetas de crédito,
  catálogos editables.
- Paginación por cursor en todas las listas (10, 25 o 50 por página).
- Montos con separador de miles automático y coma decimal.
- Reglas de categorías genéricas y personales (`reglas.txt`, no versionado).
- Estilo de vidrio con modo claro, oscuro o del sistema, "reducir transparencia" y "reducir movimiento";
  navegación completa con teclado.
- Instaladores para Linux (x64 y ARM64) y Windows (x64 y ARM64).

[Sin publicar]: https://github.com/CpuJP/sueldapp/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/CpuJP/sueldapp/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/CpuJP/sueldapp/compare/v0.1.0-alpha.2...v0.2.0
[0.1.0-alpha.2]: https://github.com/CpuJP/sueldapp/compare/v0.1.0-alpha.1...v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/CpuJP/sueldapp/releases/tag/v0.1.0-alpha.1
