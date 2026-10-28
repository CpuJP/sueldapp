# Contrato UI ↔ núcleo (comandos Tauri)

Todos devuelven JSON. Montos viajan como **string decimal** (`"12345.67"`), nunca como número.
Errores: un string con el mensaje para el usuario. Los montos de salida van como `{valor, texto, negativo}`; los montos de entrada, como el texto que escribió el usuario (`"150.000"`). Los argumentos de primer nivel van en camelCase (`reasignarA`).

| Comando | Entrada | Salida |
|---|---|---|
| `resumen_mes` | `{anio, mes}` | `{mes, ingresos[], gastos[], total, pagado, ingreso, restante, restante_pagado, proximos[]}` |
| `resumen_anio` | `{anio}` | `{anio, ingreso, total, pagado, restante, meses[]}` |
| `listar_meses` | — | `[{id, anio, mes, cerrado}]` |
| `guardar_mes` | `{id?, anio, mes, cerrado}` | `Mes` |
| `borrar_mes` | `{id}` | `MesCompleto` (para deshacer) |
| `buscar_gastos` | `{texto?, anio?, mes?, estado_id?, categoria_id?, deuda_id?, tarjeta_id?}` | `[Gasto]` |
| `guardar_gasto` | `Gasto` (sin id = crear) | `Gasto` |
| `borrar_gasto` | `{id}` | `Gasto` |
| `restaurar_gasto` | `Gasto` | `Gasto` |
| `guardar_ingreso` / `borrar_ingreso` / `restaurar_ingreso` | igual que gasto | |
| `listar_deudas` | — | `[DeudaConSaldo]` |
| `guardar_deuda` / `borrar_deuda` / `restaurar_deuda` | | |
| `listar_tarjetas` | — | `[TarjetaConSaldo]` |
| `guardar_tarjeta` / `borrar_tarjeta` / `restaurar_tarjeta` | | |
| `guardar_estado` / `borrar_estado` | `{estado}` / `{id, reasignarA?}` | `Estado` / `()` |
| `guardar_item` | `{tipo: categorias\|fuentes, item}` | `Item` |
| `borrar_item` | `{tipo, id, reasignarA?}` | `()` |
| `config` / `catalogos` | — | configuración y catálogos para formularios |
| `importar_ods` | `{forzar}` (abre el diálogo) | `Informe \| null` |
| `exportar_ods` | — (abre el diálogo) | `ruta \| null` |
| `exportar_csv` | — (abre el diálogo) | `[ruta] \| null` |
| `capturar` | `{nombre}` (solo `--features capturas`) | `ruta` |
| `historial_importaciones` | — | `[Importacion]` |
