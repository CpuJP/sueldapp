//! Importa un `.ods` desde la terminal y guarda el log en `logs/`.
//!
//! Uso: `importar <archivo.ods> [--forzar] [--db <ruta.sqlite3>]`
//!      `importar --categorizar [--db <ruta.sqlite3>]` (categoriza los gastos sin categoría)
//!      `importar --plantilla <salida.ods> [--db <ruta.sqlite3>]` (guarda la plantilla para llenar)

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut archivo = None;
    let mut forzar = false;
    let mut categorizar = false;
    let mut plantilla = None;
    let mut db = sueldapp_almacen::ruta_por_defecto();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--forzar" => forzar = true,
            "--categorizar" => categorizar = true,
            "--plantilla" => match args.next() {
                Some(r) => plantilla = Some(PathBuf::from(r)),
                None => return error("falta la ruta tras --plantilla"),
            },
            "--db" => match args.next() {
                Some(r) => db = PathBuf::from(r),
                None => return error("falta la ruta tras --db"),
            },
            _ => archivo = Some(PathBuf::from(a)),
        }
    }
    if let Some(salida) = plantilla {
        let r = sueldapp_almacen::abrir(&db)
            .map_err(sueldapp_ods::Error::from)
            .and_then(|c| sueldapp_ods::escritura::exportar_plantilla(&c, &salida));
        return match r {
            Ok(()) => {
                println!("Plantilla guardada en {}", salida.display());
                ExitCode::SUCCESS
            }
            Err(e) => error(&e.to_string()),
        };
    }
    if categorizar {
        let resultado = sueldapp_almacen::abrir(&db)
            .map_err(sueldapp_ods::Error::from)
            .and_then(|mut c| {
                let reglas = sueldapp_ods::importar::Reglas::cargar()?;
                sueldapp_ods::importar::categorizar_sin_categoria(&mut c, &reglas)
            });
        return match resultado {
            Ok(n) => {
                println!("{n} gastos categorizados en {}", db.display());
                ExitCode::SUCCESS
            }
            Err(e) => error(&e.to_string()),
        };
    }
    let Some(archivo) = archivo else {
        return error("uso: importar <archivo.ods> [--forzar] [--db <ruta.sqlite3>]");
    };
    let mut c = match sueldapp_almacen::abrir(&db) {
        Ok(c) => c,
        Err(e) => return error(&e.to_string()),
    };
    let inf = match sueldapp_ods::importar::importar_archivo(&mut c, &archivo, forzar) {
        Ok(i) => i,
        Err(e) => return error(&e.to_string()),
    };
    let segundos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let log = PathBuf::from("logs").join(format!("importacion-{segundos}.log"));
    let mut texto = format!("{}\nBase: {}\n\n", inf.resumen(), db.display());
    for d in &inf.conciliacion.diferencias {
        texto.push_str(&format!(
            "DIFERENCIA {}: hoja {:?} app {:?}\n",
            d.concepto, d.hoja, d.app
        ));
    }
    for d in &inf.conciliacion.aceptadas {
        texto.push_str(&format!(
            "ACEPTADA   {}: hoja {:?} app {:?}\n",
            d.concepto, d.hoja, d.app
        ));
    }
    texto.push('\n');
    texto.push_str(&inf.log_texto());
    if std::fs::create_dir_all("logs")
        .and_then(|_| std::fs::write(&log, &texto))
        .is_ok()
    {
        println!("Log: {}", log.display());
    }
    println!("{}", inf.resumen());
    if inf.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn error(m: &str) -> ExitCode {
    eprintln!("Error: {m}");
    ExitCode::FAILURE
}
