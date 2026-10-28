# ADR 0006 — Slint para la interfaz (reemplaza la ADR 0001)

- Estado: aceptada (2026-10-28, aprobada por el usuario)

## Contexto

La interfaz en Tauri 2 + WebKitGTK usaba unos 200–220 MiB de memoria (PSS), casi todo del motor
web: una página en blanco ya ocupaba ~197 MiB. El usuario pidió dejar WebKit y usar herramientas
nativas y eficientes sin cambiar el diseño.

## Decisión

Usar [Slint](https://slint.dev) 1.18 (backend winit, renderizador femtovg por GPU con respaldo por
software) con la licencia **Royalty-free 2.0**. La interfaz se escribe en `crates/app/ui/*.slint`
y se compila a Rust con `slint-build`; los diálogos de archivo usan `rfd` (portal XDG en Linux).

La licencia exige atribución: la pantalla Ajustes muestra «Hecho con Slint» (`AboutSlint`) y el
README lo menciona.

## Alternativas medidas (prototipo del Tablero, PSS)

| Opción | Memoria |
|---|---|
| Tauri 2 + WebKitGTK (antes) | ~200–220 MiB |
| egui | 48–54 MiB |
| Slint femtovg (GPU) | 43–56 MiB |
| Slint software (CPU) | 28 MiB |

App completa con datos reales: 52 MiB (GPU) y 36 MiB (`SLINT_BACKEND=winit-software`).

## Consecuencias

- Un solo proceso, sin motor web, binario de ~42 MB y sin dependencias de WebKit/GTK.
- Slint no tiene desenfoque del fondo (`backdrop-filter`): el vidrio son paneles translúcidos sobre
  un fondo de manchas ya difuminado. A la vista es igual porque el fondo no cambia.
- "Reducir movimiento" del sistema no se detecta: lo controla el ajuste propio de la app.
- Las capturas usan `Window::take_snapshot` (`SUELDAPP_CAPTURAS`), sin herramientas externas, e
  incluyen una prueba de humo que acciona los mismos callbacks que los botones.
- Instaladores con `cargo-packager` (deb, AppImage, NSIS, MSI) y `cargo-generate-rpm`.
