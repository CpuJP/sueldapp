# ADR 0001 — Tauri 2 con HTML/CSS/JS plano para la interfaz

- Estado: aceptada (2026-10-28, aprobada por el usuario)

## Contexto

El diseño pide superficies de vidrio con desenfoque real del fondo, modo claro/oscuro,
"reducir transparencia", "reducir movimiento" y accesibilidad AA con teclado.

## Decisión

Usar Tauri 2 con WebKitGTK y una interfaz HTML/CSS/JS sin framework ni bundler.

## Alternativas

- **Slint:** no ofrece desenfoque del fondo; solo translucidez plana.
- **iced:** no ofrece desenfoque sin shaders propios, y su soporte de lectores de pantalla está
  incompleto.
- **Tauri con React o Svelte:** añade npm y cientos de dependencias para 7 pantallas.

## Consecuencias

- Hay `backdrop-filter`, ARIA y `prefers-*` nativos.
- El vidrio desenfoca el contenido de la ventana, no el escritorio que hay detrás.
- Las capturas necesitan la API de WebKit (`--features capturas`), porque las herramientas de X
  no leen la superficie GL.
