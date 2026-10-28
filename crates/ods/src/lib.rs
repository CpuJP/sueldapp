//! Lectura y escritura de `.ods`, importador de la hoja original, conciliación y exportación.

pub mod conciliar;
pub mod csv;
pub mod escritura;
pub mod formula;
pub mod hoja;
pub mod importar;
pub mod lectura;
pub mod plantilla;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Formato(String),
    #[error("No se pudo leer o escribir el archivo: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Almacen(#[from] sueldapp_almacen::Error),
}

impl From<sueldapp_dominio::Invalido> for Error {
    fn from(e: sueldapp_dominio::Invalido) -> Self {
        Error::Formato(e.0)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Almacen(e.into())
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error::Formato(format!("Error del archivo comprimido: {e}"))
    }
}
