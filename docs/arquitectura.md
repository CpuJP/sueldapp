# Arquitectura

Sueldapp es una app de escritorio local. El núcleo está escrito en Rust y la interfaz es nativa,
con Slint (un solo proceso, sin motor web). No hay servidor ni red: los datos viven en
un archivo SQLite del usuario.

```mermaid
flowchart LR
  subgraph UI["crates/app/ui (Slint)"]
    V[Pantallas: Resumen, Movimientos, Deudas,<br/>Tarjetas, Catálogos, Importar/Exportar, Ajustes]
  end
  subgraph APP["crates/app"]
    C[vistas.rs · pantallas.rs · acciones.rs<br/>validación de entrada y modelos]
    N[notificaciones.rs<br/>avisos y deshacer]
    D[demo.rs · capturas.rs<br/>datos ficticios, capturas y prueba de humo]
  end
  subgraph NUCLEO["Núcleo Rust"]
    DOM[crates/dominio<br/>tipos, cálculos, formato COP]
    ALM[crates/almacen<br/>SQLite, migraciones, repositorios]
    ODS[crates/ods<br/>lector/escritor .ods, importador,<br/>conciliación, CSV]
  end
  DB[(sueldapp.sqlite3<br/>~/.local/share/sueldapp)]
  HOJA[/Ingreso-Egreso.ods<br/>solo lectura/]

  V -- callbacks y modelos --> C
  C --> DOM
  C --> ALM
  C --> ODS
  C --> N
  N -- avisos --> V
  ODS --> DOM
  ODS --> ALM
  ALM --> DOM
  ALM --> DB
  HOJA --> ODS
```

## Capas

| Crate | Responsabilidad | Depende de |
|---|---|---|
| `dominio` | Tipos (`Mes`, `Gasto`, `Ingreso`, `Deuda`, `Tarjeta`, `Estado`), reglas de la hoja (Total, Pagado, Restante, saldos), formato y lectura de montos, normalización de texto. Sin IO. | `rust_decimal`, `serde` |
| `almacen` | Apertura de SQLite (WAL, claves foráneas), migraciones con `PRAGMA user_version`, repositorios CRUD con *upsert* por `origen` y deshacer. | `dominio`, `rusqlite` |
| `ods` | Lectura del XML del `.ods` sin `f64`, fórmulas simples, extracción de la pestaña `Nómina`, importador transaccional con log, conciliación, exportación `.ods`/`.csv` y restauración. Binario `importar`. | `dominio`, `almacen`, `zip`, `quick-xml` |
| `app` | Interfaz Slint, diálogos de archivo nativos (`rfd`), enlace con el núcleo. Todas las notificaciones pasan por `src/notificaciones.rs` (tipo, duración y deshacer). | todos, `slint`, `rfd` |

## Reglas clave

- El dinero es siempre `Decimal`: en SQLite se guarda como `TEXT`. La interfaz recibe textos ya
  formateados y nunca formatea ni suma; los gráficos reciben geometría calculada en `geometria.rs`.
- La interfaz no calcula nada: cada total sale de `dominio::calculos`.
- Ninguna dependencia usa la red.
- Los gastos tienen `origen` (la celda de la hoja). Esa es la clave de idempotencia del importador.

## Datos en disco

- Base: `$SUELDAPP_DB`, o `$XDG_DATA_HOME/sueldapp/sueldapp.sqlite3`
  (por defecto `~/.local/share/sueldapp/sueldapp.sqlite3`).
- Ajustes de apariencia: `ajustes.txt` junto a la base.
- Logs del binario `importar`: `logs/` (excluido de git).

Decisiones: ver `docs/decisiones/`.
