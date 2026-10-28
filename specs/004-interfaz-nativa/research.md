# Research: interfaz nativa

## Medición (prototipo con barra lateral, 3 paneles translúcidos, gráfico de línea, lista de 50 filas)

Linux x64 (Fedora, Wayland), build release, 2 corridas, PSS total de todos los procesos:

| Interfaz | PSS | Memoria privada | Procesos |
|---|---:|---:|---:|
| Tauri 2 + WebKitGTK (app actual) | 200–220 MiB | ~230 MiB | 3 |
| Tauri 2 + WebKitGTK, página en blanco | ~197 MiB | — | 3 |
| Slint 1.18, renderizador femtovg (OpenGL) | 43–56 MiB | 37–50 MiB | 1 |
| Slint 1.18, renderizador por software | 28 MiB | 19 MiB | 1 |
| egui/eframe 0.37 (glow) | 48–54 MiB | 41–48 MiB | 1 |

## Decisión propuesta: Slint

- **Diseño fiel**: lenguaje declarativo con propiedades parecidas a CSS (`border-radius`,
  gradientes, `drop-shadow-*`, opacidad, animaciones con `animate`, estados `has-hover`/`has-focus`).
  Traducir `estilos.css` es directo; en egui cada detalle visual es código de pintado a mano.
- **Rendimiento**: el más bajo medido (28 MiB por software); por GPU en equipos con buena gráfica.
- **Accesibilidad**: AccessKit (lectores de pantalla en Linux y Windows), navegación con teclado y foco.
- **Plataformas**: Linux y Windows en x64 y ARM64; empaquetado con `cargo-packager`/scripts propios
  (`.deb`, `.rpm`, AppImage, instalador de Windows).
- **Vidrio**: Slint no desenfoca lo que hay detrás de un elemento. Como el fondo son manchas de color
  ya difuminadas y estáticas, se pinta una vez como gradientes radiales; los paneles translúcidos con
  borde especular y sombra sobre ese fondo se ven igual que hoy.

### Licencia (decisión del usuario)

Slint tiene licencia doble: **Royalty-free 2.0** (gratis para apps de escritorio, exige atribución
visible: el widget `AboutSlint` o el distintivo "Made with Slint" en la app/README) o **GPLv3**
(los binarios distribuidos quedan bajo GPL). Nuestro código sigue siendo MIT en ambos casos.

## Alternativas

- **egui**: MIT/Apache sin condiciones; memoria similar a Slint-GPU. Modo inmediato: el diseño
  actual (paneles, tablas, calendario, gráficos) habría que pintarlo a mano; menos fiel.
- **iced**: sin AccessKit (lectores de pantalla) a la fecha; descartado por accesibilidad.
- **GTK4 + libadwaita**: nativo en Linux, pobre en Windows ARM y con estética Adwaita.
