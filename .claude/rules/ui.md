# Reglas de interfaz

- HTML/CSS/JS plano en `crates/app/ui/`. Sin npm, sin CDN, sin fuentes remotas.
- Tokens de color en `:root` con variantes `prefers-color-scheme: dark` y `[data-transparencia="reducida"]`.
- Vidrio: `backdrop-filter: blur() saturate()`, borde con brillo especular sutil, sombras con
  profundidad. Con reducir transparencia: superficies sólidas.
- Transiciones ≤ 200 ms; ninguna con `prefers-reduced-motion: reduce`.
- Cifras con `font-variant-numeric: tabular-nums`, alineadas a la derecha.
- Prohibido: fondos crema/blanco roto planos, gradientes morado-azul, emojis como iconos,
  etiquetas "01/02/03", monospace decorativo, cuadrícula de tarjetas idénticas como única estructura.
- Accesibilidad: AA, foco visible, `<dialog>` modal con Escape, etiquetas en todos los controles,
  estado con texto además de color.
- Lenguaje sencillo, en español.
