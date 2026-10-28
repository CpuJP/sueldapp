# Implementation Plan: Interfaz nativa (Slint)

**Spec**: [spec.md](spec.md) · **Research**: [research.md](research.md)

## Estructura

```text
crates/app/                  (se reemplaza: sin Tauri, sin ui/ web)
├── build.rs                 slint-build compila ui/*.slint
├── ui/
│   ├── tema.slint           tokens claro/oscuro/reducir transparencia (de estilos.css)
│   ├── componentes.slint    Vidrio, Boton, Campo, Selector, Chip, Paginador, Dialogo, Aviso…
│   ├── graficos.slint       barras agrupadas, divergentes, línea, barras horizontales
│   ├── calendario.slint     selector de periodo por meses
│   └── pantallas/*.slint    tablero, resumen, movimientos, deudas, tarjetas, catalogos, datos, ajustes
└── src/
    ├── main.rs              ventana, estado, enlace con el núcleo
    ├── vistas.rs            modelos de las pantallas (equivalen a comandos.rs de hoy)
    ├── dialogos.rs          rfd (diálogos nativos de archivo)
    └── capturas.rs          capturas por software (sin X11) para docs
```

## Fases

1. Tema y componentes base; ventana con barra lateral y navegación (Alt+1…8).
2. Pantallas en orden de uso: Tablero, Resumen, Movimientos, Deudas, Tarjetas, Catálogos, Datos, Ajustes.
3. Diálogos (editar/borrar con deshacer), máscara de montos, calendario, gráficos con tooltip.
4. Capturas en los 4 modos y comparación lado a lado con la versión web; prueba de humo.
5. Empaquetado nativo en CI (Linux x64/ARM64, Windows x64/ARM64), docs y ADR 0006.

## Constitución

Sin cambios de principios. Se elimina la CSP (ya no hay web) y la red sigue sin usarse. ADR 0006
reemplaza la ADR 0001.
