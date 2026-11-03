# Spec: Interfaz adaptable

Pedido del usuario: que toda la interfaz se adapte a cualquier pantalla y a cualquier tamaño de
ventana, y que el menú se pueda compactar a solo íconos y volver a su forma normal con un botón.

## Requisitos

- FR-1001: La ventana se puede reducir hasta 360 × 420 puntos sin cortar contenido ni controles; lo
  que no cabe se desplaza.
- FR-1002: Tres disposiciones según el ancho del contenido: ancha (≥ 920), media (≥ 620) y pequeña.
  En media y pequeña, los paneles lado a lado se apilan y las acciones largas pasan a solo ícono,
  con su nombre accesible.
- FR-1003: El menú se compacta y se expande con un botón y con Alt + M, y la preferencia se guarda.
  Con la ventana por debajo de 820 se compacta solo, y el botón lo abre encima del contenido; se
  cierra con Escape, al elegir una sección o al pulsar fuera.
- FR-1004: Compacto, cada sección muestra su nombre al pasar el puntero o al enfocarla con teclado.
- FR-1005: El Tablero reparte sus filas según la disposición sin cambiar lo que eligió la persona.
- FR-1006: La prueba de humo recorre todas las pantallas en seis tamaños, de 360 × 640 a 1920 × 1080.
