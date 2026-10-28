# Reglas de interfaz

- Slint en `crates/app/ui/*.slint` (estilo fluent). Sin recursos remotos ni fuentes descargadas.
- Tokens de color en el global `Tema` (`tema.slint`), con variantes claro, oscuro y `solido`
  (reducir transparencia).
- Vidrio: paneles translúcidos sobre el fondo difuminado, borde con brillo especular sutil, sombras
  con profundidad. Con reducir transparencia: superficies sólidas.
- Transiciones ≤ 200 ms; ninguna con `Tema.sin-movimiento`.
- Cifras alineadas a la derecha; la UI muestra los textos de monto que da el núcleo.
- Prohibido: fondos crema/blanco roto planos, gradientes morado-azul, emojis como iconos,
  etiquetas "01/02/03", monospace decorativo, cuadrícula de tarjetas idénticas como única estructura.
- Accesibilidad: AA, foco visible, diálogo modal con Escape, `accessible-label` en todos los
  controles, estado con texto además de color.
- Lenguaje sencillo, en español.
