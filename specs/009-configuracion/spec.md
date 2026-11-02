# Spec: Alertas y Tablero configurables

Pedido del usuario: alertas configurables y un Tablero editable y modular: qué se muestra, cómo,
tamaño, distribución y demás.

## Requisitos

- FR-901: En Ajustes se activan o desactivan las alertas y se eligen los días de anticipación, si
  incluyen vencidos, la notificación emergente y el botón del menú.
- FR-902: El Tablero se compone de módulos, y cada uno se puede mostrar u ocultar y mover.
- FR-903: Cada módulo tiene ancho (1/3, 1/2, 2/3 o completo) y, si aplica, tamaño, vista y título.
- FR-904: Los módulos se reparten en filas de hasta 12 doceavos, en su orden. El resumen del periodo
  usa al menos la mitad; las columnas de cifras se limitan según el ancho.
- FR-905: El espacio entre módulos se puede cambiar, y Restablecer, con confirmación, vuelve al inicio.
- FR-906: Todo se guarda en archivos junto a la base, no en los datos financieros ni en las
  exportaciones. Sin archivo, el Tablero se ve como siempre.

- FR-907: La edición es directa sobre el tablero: arrastrar un módulo lo cambia de lugar, sus bordes
  cambian ancho y tamaño con vista previa, − lo quita y Añadir módulo lo recupera.
- FR-908: Editar abre los ajustes del módulo, con los que todo se puede hacer también con teclado.
