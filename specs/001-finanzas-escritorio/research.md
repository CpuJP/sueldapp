# Research: App de escritorio de finanzas personales

> **Nota (2026-10-28):** la interfaz Tauri/HTML descrita aquí fue reemplazada por una interfaz
> nativa con Slint. Ver [`specs/004-interfaz-nativa`](../004-interfaz-nativa/) y la ADR 0006.

## R1. Framework de UI (efecto vidrio real)

**Decision**: Tauri 2 (núcleo Rust + interfaz HTML/CSS/JS sin bundler).

**Rationale**:
- El efecto vidrio pide desenfoque y saturación **de lo que hay detrás** de una superficie.
  En la webview (WebKitGTK 2.54 en Fedora, ya instalado) eso es CSS `backdrop-filter:
  blur() saturate()`, con composición por GPU, capas reales y transiciones baratas.
- Modo claro/oscuro (`prefers-color-scheme`), "reducir movimiento" (`prefers-reduced-motion`),
  foco con teclado, lectores de pantalla (ARIA → AT-SPI) y cifras tabulares
  (`font-variant-numeric: tabular-nums`) vienen de la plataforma web.
- Todo el cálculo sigue en Rust; la interfaz solo llama comandos.
- Sin red: los recursos se sirven desde el binario (protocolo propio de Tauri) y la CSP bloquea
  cualquier origen externo.

**Alternatives considered**:
- **Slint**: UI declarativa nativa, ligera y con buena accesibilidad. No tiene desenfoque del
  fondo (solo sombras y opacidad). El vidrio sería una imitación con colores semitransparentes
  sin desenfoque. Rechazado por el requisito de diseño.
- **iced**: Rust puro, estilo Elm. No trae blur de fondo; habría que escribir shaders wgpu propios
  para cada superficie, y su accesibilidad (lectores de pantalla) sigue incompleta. Rechazado
  por coste y accesibilidad.
- **Tauri + framework JS (React/Svelte)**: añade npm, bundler y cientos de dependencias para
  ~6 pantallas. Rechazado por simplicidad; HTML/JS plano basta.

Límite conocido: el vidrio desenfoca el contenido de la propia ventana (fondo y capas de la app),
no el escritorio que hay detrás de la ventana. Desenfocar el escritorio exige compositor y APIs
propias de cada sistema; queda fuera.

## R2. Persistencia

**Decision**: SQLite con `rusqlite` (feature `bundled`), migraciones versionadas propias con
`PRAGMA user_version` y SQL embebido (`include_str!`).

**Rationale**: una sola persona, un solo archivo, sin servidor. `rusqlite` es síncrono y simple;
`sqlx` aporta async y comprobación en compilación que aquí no hacen falta. Un framework de
migraciones (`refinery`) es innecesario para una lista ordenada de archivos `.sql`.

Base en `~/.local/share/sueldapp/sueldapp.sqlite3` (directorio de datos del usuario). Modo WAL y
`foreign_keys = ON`.

## R3. Dinero

**Decision**: `rust_decimal::Decimal`; en SQLite como `TEXT` (p. ej. `"12345.67"`).

**Rationale**: exactitud sin conversión. Guardar en centavos `INTEGER` también sería exacto, pero
el texto se lee directo en exportes y depuración. Ningún cálculo en SQL: todo en el dominio.

## R4. Lectura y escritura `.ods`

**Decision**: leer y escribir el XML del `.ods` directamente con `zip` + `quick-xml`.

**Rationale**:
- Necesitamos el texto exacto de `office:value` (para convertir a `Decimal` sin pasar por `f64`),
  las fórmulas (`table:formula`) para deducir vínculos de cuotas y ajustes, y las celdas
  combinadas/repetidas. `calamine` entrega números como `f64`; `spreadsheet-ods` también usa `f64`.
- El escritor genera un `.ods` mínimo válido (mimetype, manifest, content.xml) con valores
  escritos desde `Decimal`.

## R5. Importador: estrategia de mapeo

**Decision**: el importador recorre la pestaña `Nómina` por franjas de año (F, K, P, U) y por
bloques (`Sueldo <Mes>` → `Total` → `Sueldo` → `Restante` → `Prima`). La celda de origen
(`Nómina!L163`) es la clave natural de cada registro: con ella se hace *upsert* (idempotencia).

- Vínculo cuota → deuda: las referencias de la fórmula de la columna B; las constantes de la fórmula
  son el ajuste manual.
- Vínculo por nombre (solo cuotas **no pagadas**, para no cambiar ningún total): raíz del concepto
  sin "Cuota N", "FINAL", "Mitad", números y guiones. Ej. `Consola XL – Cuota 6` → `consola xl`.
- Deuda sin fórmula de pago (decisión del usuario): Total = suma de sus cuotas vinculadas por nombre
  (alias configurado en `reglas.txt`).
- Tarjetas: palabra del concepto → tarjeta, según `reglas.txt` (privado).
- Categorías: pagos de tarjeta → Tarjetas; el resto según reglas genéricas + `reglas.txt`.
- Gastos fuera de bloque → mes del siguiente encabezado de su franja (o el anterior si no hay), con
  marca `fuera_de_bloque` (no cuentan en totales).
- Año del bloque = año de la franja (corrige encabezados con año equivocado y lo registra).

## R6. Conciliación

**Decision**: el importador guarda los valores calculados por la hoja (Total, SubTotal, Sueldo,
Restante, SubRestante, totales anuales, Pago/Total/Pendiente de deudas) y los compara, redondeados
a 2 decimales, con lo que calcula el dominio sobre lo importado. Diferencias esperadas: solo la
deudas sin fórmula de pago (lista blanca explícita). Prueba de integración `conciliacion_ods_real` en
`crates/ods/tests/`, que lee el `.ods` de la raíz (o `SUELDAPP_ODS`). Sin el archivo la prueba
falla, salvo con `SUELDAPP_SIN_ODS=1` (máquinas sin los datos).

## R7. Capturas de pantalla

**Decision**: la app acepta `SUELDAPP_DB` (ruta de la base) y `SUELDAPP_DEMO=1` (llena una base
vacía con datos ficticios desde Rust), y `SUELDAPP_TEMA=claro|oscuro` para forzar el tema en
capturas. Las capturas se toman de la ventana real. Los datos reales nunca entran en `docs/`.

Alternativa rechazada: un backend falso en JS, porque duplicaría las reglas de dinero fuera del
dominio (Constitución III).

## R8. Diálogos de archivo

**Decision**: `tauri-plugin-dialog` (diálogos nativos GTK, sin red).
