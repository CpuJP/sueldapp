//! Conciliación contra el `Ingreso-Egreso.ods` real (no versionado). Busca el archivo en la raíz
//! del repositorio o en `SUELDAPP_ODS`. Sin archivo falla, salvo con `SUELDAPP_SIN_ODS=1`.

use std::path::PathBuf;
use sueldapp_ods::importar::{importar_bytes_con, Accion, Reglas};

fn ods_real() -> Option<Vec<u8>> {
    let ruta = std::env::var_os("SUELDAPP_ODS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Ingreso-Egreso.ods")
        });
    match std::fs::read(&ruta) {
        Ok(b) => Some(b),
        Err(_) if std::env::var_os("SUELDAPP_SIN_ODS").is_some() => {
            eprintln!("SUELDAPP_SIN_ODS: se omite la conciliación");
            None
        }
        Err(e) => panic!(
            "No se encontró {} ({e}). Pon el .ods en la raíz, define SUELDAPP_ODS o SUELDAPP_SIN_ODS=1.",
            ruta.display()
        ),
    }
}

/// Reglas personales (no versionadas) junto al `.ods`: `reglas.txt` en la raíz o `SUELDAPP_REGLAS`.
fn reglas() -> Reglas {
    let ruta = std::env::var_os("SUELDAPP_REGLAS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reglas.txt"));
    std::fs::read_to_string(ruta)
        .map(|t| Reglas::desde_texto(&t).unwrap())
        .unwrap_or_else(|_| Reglas::genericas())
}

fn importar_bytes(
    c: &mut sueldapp_almacen::Connection,
    n: &str,
    b: &[u8],
    forzar: bool,
) -> Result<sueldapp_ods::importar::Informe, sueldapp_ods::Error> {
    importar_bytes_con(c, n, b, forzar, &reglas())
}

#[test]
fn conciliacion_ods_real() {
    let Some(bytes) = ods_real() else { return };
    let huella_antes = sueldapp_ods::importar::huella(&bytes);
    let mut c = sueldapp_almacen::abrir_memoria().unwrap();

    let inf = importar_bytes(&mut c, "Ingreso-Egreso.ods", &bytes, false).unwrap();
    for d in &inf.conciliacion.diferencias {
        eprintln!(
            "DIFERENCIA {}: hoja {:?} app {:?}",
            d.concepto, d.hoja, d.app
        );
    }
    assert!(inf.ok, "{}", inf.resumen());
    assert!(inf.conciliacion.diferencias.is_empty());
    assert!(inf.conciliacion.comparados > 0);
    assert!(inf.conteos.meses > 0 && inf.conteos.deudas > 0);
    // Las únicas diferencias aceptadas son de deudas sin fórmula de pago (decisión del usuario).
    assert!(inf
        .conciliacion
        .aceptadas
        .iter()
        .all(|d| d.concepto.contains("deuda")));
    // Las pestañas distintas de Nómina quedan registradas como omitidas.
    assert!(inf
        .log
        .iter()
        .any(|l| !l.origen.starts_with("Nómina!") && l.accion == Accion::Omitida));

    // Idempotencia: mismo archivo → nada cambia; forzado → mismos conteos.
    let antes = sueldapp_ods::importar::conteos_base(&c).unwrap();
    let otra = importar_bytes(&mut c, "Ingreso-Egreso.ods", &bytes, false).unwrap();
    assert!(otra.ya_importado);
    let forzada = importar_bytes(&mut c, "Ingreso-Egreso.ods", &bytes, true).unwrap();
    assert!(forzada.ok, "{}", forzada.resumen());
    assert_eq!(sueldapp_ods::importar::conteos_base(&c).unwrap(), antes);

    // Exportar lo importado y restaurarlo en una base vacía conserva los totales.
    let mut exportado = std::io::Cursor::new(Vec::new());
    sueldapp_ods::escritura::escribir_ods(
        &sueldapp_ods::escritura::tablas(&c).unwrap(),
        &mut exportado,
    )
    .unwrap();
    let mut copia = sueldapp_almacen::abrir_memoria().unwrap();
    let r = importar_bytes(&mut copia, "export.ods", exportado.get_ref(), false).unwrap();
    assert!(r.ok, "{}", r.resumen());
    assert_eq!(sueldapp_ods::importar::conteos_base(&copia).unwrap(), antes);

    // El original no cambia.
    let despues = std::fs::read(
        std::env::var_os("SUELDAPP_ODS")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Ingreso-Egreso.ods")
            }),
    )
    .unwrap();
    assert_eq!(sueldapp_ods::importar::huella(&despues), huella_antes);
}

#[test]
fn archivo_invalido_no_escribe_nada() {
    let mut c = sueldapp_almacen::abrir_memoria().unwrap();
    assert!(importar_bytes(&mut c, "x.ods", b"no soy un zip", false).is_err());
    assert_eq!(sueldapp_ods::importar::conteos_base(&c).unwrap().gastos, 0);
}
