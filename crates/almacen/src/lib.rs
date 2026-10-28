//! Persistencia SQLite: apertura, migraciones versionadas y repositorios.

pub mod repo;

pub use rusqlite::Connection;
use std::path::Path;

/// Migraciones en orden. `PRAGMA user_version` guarda cuántas se aplicaron.
/// Nunca edites una migración ya publicada: añade una nueva al final.
const MIGRACIONES: &[&str] = &[include_str!("../migraciones/0001_inicial.sql")];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Validacion(String),
    #[error("No se encontró el registro.")]
    NoEncontrado,
    #[error("Error de la base de datos: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

impl From<sueldapp_dominio::Invalido> for Error {
    fn from(e: sueldapp_dominio::Invalido) -> Self {
        Error::Validacion(e.0)
    }
}

pub type Resultado<T> = Result<T, Error>;

/// Ruta de la base: `SUELDAPP_DB`, o la carpeta de datos del usuario + `sueldapp/sueldapp.sqlite3`:
/// Linux `~/.local/share` (o `$XDG_DATA_HOME`), Windows `%APPDATA%`, macOS `~/Library/Application Support`.
pub fn ruta_por_defecto() -> std::path::PathBuf {
    if let Some(r) = std::env::var_os("SUELDAPP_DB") {
        return r.into();
    }
    let datos = if cfg!(windows) {
        // %APPDATA% (C:\Users\<usuario>\AppData\Roaming)
        std::env::var_os("APPDATA").map(std::path::PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|h| std::path::Path::new(&h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".local/share"))
            })
    }
    .unwrap_or_else(|| std::path::PathBuf::from("."));
    datos.join("sueldapp").join("sueldapp.sqlite3")
}

/// Abre (o crea) la base en `ruta` y aplica las migraciones pendientes.
pub fn abrir(ruta: &Path) -> Resultado<Connection> {
    if let Some(dir) = ruta.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| Error::Validacion(format!("No se pudo crear {}: {e}", dir.display())))?;
    }
    let c = Connection::open(ruta)?;
    // WAL: un cierre inesperado no pierde transacciones confirmadas.
    c.pragma_update(None, "journal_mode", "WAL")?;
    c.pragma_update(None, "synchronous", "NORMAL")?;
    preparar(c)
}

/// Base en memoria (pruebas).
pub fn abrir_memoria() -> Resultado<Connection> {
    preparar(Connection::open_in_memory()?)
}

fn preparar(mut c: Connection) -> Resultado<Connection> {
    c.pragma_update(None, "foreign_keys", "ON")?;
    migrar(&mut c)?;
    Ok(c)
}

fn migrar(c: &mut Connection) -> Resultado<()> {
    let aplicadas: i64 = c.query_row("PRAGMA user_version", [], |f| f.get(0))?;
    for (i, sql) in (0i64..).zip(MIGRACIONES).skip(aplicadas.max(0) as usize) {
        let tx = c.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i + 1)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migra_una_sola_vez() {
        let dir = std::env::temp_dir().join(format!("sueldapp-test-{}", std::process::id()));
        let ruta = dir.join("prueba.sqlite3");
        let _ = std::fs::remove_file(&ruta);
        drop(abrir(&ruta).unwrap());
        let c = abrir(&ruta).unwrap();
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM estados", [], |f| f.get(0))
            .unwrap();
        assert_eq!(n, 5);
        drop(c);
        let _ = std::fs::remove_dir_all(dir);
    }
}
