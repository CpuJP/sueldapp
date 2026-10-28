//! Exportación CSV: un archivo por tabla, UTF-8 con BOM, montos con punto decimal.

use crate::escritura::{tablas, Dato};
use crate::Error;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

fn campo(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn dato(d: &Dato) -> String {
    match d {
        Dato::Texto(t) => campo(t),
        Dato::Numero(n) => n.normalize().to_string(),
        Dato::Vacio => String::new(),
    }
}

/// Escribe `<carpeta>/<tabla>.csv` para cada tabla y devuelve las rutas.
pub fn exportar_csv(c: &Connection, carpeta: &Path) -> Result<Vec<PathBuf>, Error> {
    std::fs::create_dir_all(carpeta)?;
    let mut rutas = Vec::new();
    for t in tablas(c)? {
        let mut s = String::from("\u{feff}");
        s.push_str(
            &t.encabezados
                .iter()
                .map(|e| campo(e))
                .collect::<Vec<_>>()
                .join(","),
        );
        s.push_str("\r\n");
        for f in &t.filas {
            s.push_str(&f.iter().map(dato).collect::<Vec<_>>().join(","));
            s.push_str("\r\n");
        }
        let ruta = carpeta.join(format!("{}.csv", t.nombre.to_lowercase()));
        std::fs::write(&ruta, s)?;
        rutas.push(ruta);
    }
    Ok(rutas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comillas() {
        assert_eq!(campo("Donde Ana's"), "Donde Ana's");
        assert_eq!(campo("a, b"), "\"a, b\"");
        assert_eq!(campo("di \"hola\""), "\"di \"\"hola\"\"\"");
    }
}
