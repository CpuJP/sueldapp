# Feature Specification: Interfaz nativa de bajo consumo

**Feature Branch**: `004-interfaz-nativa`

**Created**: 2026-10-28

**Status**: Plan pendiente de aprobación

**Input**: "Ya no usemos WebKit, usemos herramientas nativas, eficientes, optimizadas, que no consuman
tantos recursos. Dejemos el desenfoque real de vidrio y usemos un blur normal nativo y eficiente.
Prioricemos la optimización y código nativo, sin dejar aparte la estética: que el cambio de
tecnología no cambie el diseño actual."

## User Scenarios & Testing

### US1 — Misma app, mucha menos memoria (P1)

Abro Sueldapp y veo las mismas 8 pantallas, con el mismo diseño, colores, tipografía y
comportamiento, pero la app usa una fracción de la memoria y arranca más rápido.

**Acceptance**:

1. Memoria total (PSS) en reposo con datos reales ≤ 80 MiB (hoy ~200–220 MiB).
2. Un solo proceso (hoy 3: ventana, página web y red de WebKit).
3. Arranque hasta ver el tablero < 1 s.
4. Todas las funciones actuales siguen: tablero con gráficos y calendario, resumen, movimientos con
   filtros y paginación por cursor, deudas, tarjetas, catálogos, importar/exportar, plantilla,
   ajustes (tema, reducir transparencia/movimiento), deshacer, atajos de teclado.

### US2 — Estética conservada (P1)

Paneles translúcidos con borde especular y sombra, esquinas redondeadas, fondo con manchas de
color difuminadas, modo claro/oscuro/sistema, cifras tabulares. El desenfoque *del fondo detrás de
cada panel* se sustituye por un fondo ya difuminado (pintado una vez) y paneles translúcidos: el
resultado visual es equivalente porque el fondo es liso.

### US3 — Accesibilidad sin retroceso (P1)

Navegación completa con teclado, foco visible, lectores de pantalla (AccessKit), contraste AA.

## Requirements

- **FR-401**: Sin motor web: ni WebKit ni WebView2 en ninguna plataforma.
- **FR-402**: El núcleo (`dominio`, `almacen`, `ods`) no cambia; la interfaz llama a sus funciones
  directamente, sin JSON ni IPC.
- **FR-403**: Linux y Windows, x64 y ARM64, con los mismos instaladores.
- **FR-404**: Sin red en tiempo de ejecución (se mantiene).
- **FR-405**: Capturas de las pantallas para la documentación sin herramientas externas.

## Out of scope

Rediseño visual, funciones nuevas.
