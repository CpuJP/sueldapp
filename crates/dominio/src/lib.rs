//! Tipos y reglas de negocio de Sueldapp. Sin IO: todo cálculo de dinero vive aquí.

pub mod calculos;
pub mod estadisticas;
pub mod formato;
pub mod paginacion;
pub mod texto;

pub use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Monto exacto en COP. Nunca `f64`.
pub type Dinero = Decimal;

/// Año y mes (1–12).
pub type Periodo = (i32, u32);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mes {
    pub id: Option<i64>,
    pub anio: i32,
    pub mes: u32,
    pub cerrado: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Estado {
    pub id: Option<i64>,
    pub nombre: String,
    /// Si cuenta para el "pagado" (SubTotal) del mes y de las deudas.
    pub cuenta_pagado: bool,
    pub orden: i64,
}

/// Elemento de un catálogo simple (categorías, fuentes de ingreso).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: Option<i64>,
    pub nombre: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gasto {
    pub id: Option<i64>,
    pub mes_id: i64,
    pub concepto: String,
    pub monto: Dinero,
    pub estado_id: i64,
    pub categoria_id: Option<i64>,
    pub deuda_id: Option<i64>,
    pub tarjeta_id: Option<i64>,
    /// Fórmula original de la hoja, p. ej. `=49900*2`.
    pub desglose: Option<String>,
    pub nota: String,
    /// Fila de la hoja fuera de cualquier bloque mensual: no cuenta en los totales del mes.
    pub fuera_de_bloque: bool,
    pub orden: i64,
    /// Celda de origen en el `.ods` (`Nómina!L163`); `None` si se creó en la app.
    pub origen: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ingreso {
    pub id: Option<i64>,
    pub mes_id: i64,
    pub fuente_id: i64,
    pub monto: Dinero,
    pub desglose: Option<String>,
    pub nota: String,
    pub origen: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deuda {
    pub id: Option<i64>,
    pub nombre: String,
    pub total: Option<Dinero>,
    /// Monto que se suma al pagado sin cuota asociada (ajustes de la hoja).
    pub ajuste: Dinero,
    pub saldada: bool,
    pub notas: String,
    pub origen: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tarjeta {
    pub id: Option<i64>,
    pub nombre: String,
    pub cupo: Option<Dinero>,
    pub saldo: Option<Dinero>,
    pub dia_corte: Option<u8>,
    pub dia_pago: Option<u8>,
    pub origen: Option<String>,
}

/// Error de validación con mensaje para el usuario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalido(pub String);

impl std::fmt::Display for Invalido {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Invalido {}

fn no_vacio(texto: &str, campo: &str) -> Result<(), Invalido> {
    if texto.trim().is_empty() {
        return Err(Invalido(format!("Escribe {campo}.")));
    }
    Ok(())
}

fn no_negativo(monto: Dinero, campo: &str) -> Result<(), Invalido> {
    if monto.is_sign_negative() && !monto.is_zero() {
        return Err(Invalido(format!("{campo} no puede ser negativo.")));
    }
    Ok(())
}

fn dia_valido(dia: Option<u8>, campo: &str) -> Result<(), Invalido> {
    match dia {
        Some(d) if !(1..=31).contains(&d) => {
            Err(Invalido(format!("{campo} debe estar entre 1 y 31.")))
        }
        _ => Ok(()),
    }
}

impl Mes {
    pub fn validar(&self) -> Result<(), Invalido> {
        if !(2000..=2100).contains(&self.anio) {
            return Err(Invalido("El año debe estar entre 2000 y 2100.".into()));
        }
        if !(1..=12).contains(&self.mes) {
            return Err(Invalido("El mes debe estar entre 1 y 12.".into()));
        }
        Ok(())
    }
}

impl Gasto {
    pub fn validar(&self) -> Result<(), Invalido> {
        no_vacio(&self.concepto, "el concepto")?;
        no_negativo(self.monto, "El monto")
    }
}

impl Ingreso {
    pub fn validar(&self) -> Result<(), Invalido> {
        no_negativo(self.monto, "El monto")
    }
}

impl Deuda {
    pub fn validar(&self) -> Result<(), Invalido> {
        no_vacio(&self.nombre, "el nombre de la deuda")?;
        if let Some(t) = self.total {
            no_negativo(t, "El total")?;
        }
        Ok(())
    }
}

impl Tarjeta {
    pub fn validar(&self) -> Result<(), Invalido> {
        no_vacio(&self.nombre, "el nombre de la tarjeta")?;
        if let Some(c) = self.cupo {
            no_negativo(c, "El cupo")?;
        }
        dia_valido(self.dia_corte, "El día de corte")?;
        dia_valido(self.dia_pago, "El día de pago")
    }
}

impl Item {
    pub fn validar(&self) -> Result<(), Invalido> {
        no_vacio(&self.nombre, "un nombre")
    }
}

impl Estado {
    pub fn validar(&self) -> Result<(), Invalido> {
        no_vacio(&self.nombre, "un nombre")
    }
}

pub const NOMBRES_MES: [&str; 12] = [
    "Enero",
    "Febrero",
    "Marzo",
    "Abril",
    "Mayo",
    "Junio",
    "Julio",
    "Agosto",
    "Septiembre",
    "Octubre",
    "Noviembre",
    "Diciembre",
];

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn gasto(concepto: &str, monto: Dinero) -> Gasto {
        Gasto {
            id: None,
            mes_id: 1,
            concepto: concepto.into(),
            monto,
            estado_id: 1,
            categoria_id: None,
            deuda_id: None,
            tarjeta_id: None,
            desglose: None,
            nota: String::new(),
            fuera_de_bloque: false,
            orden: 0,
            origen: None,
        }
    }

    #[test]
    fn valida_gasto() {
        assert!(gasto("Internet", dec!(12500)).validar().is_ok());
        assert!(gasto("  ", dec!(1)).validar().is_err());
        assert!(gasto("Tigo", dec!(-1)).validar().is_err());
    }

    #[test]
    fn valida_mes_y_tarjeta() {
        let m = Mes {
            id: None,
            anio: 2026,
            mes: 13,
            cerrado: false,
        };
        assert!(m.validar().is_err());
        let t = Tarjeta {
            id: None,
            nombre: "Tarjeta Azul".into(),
            cupo: None,
            saldo: None,
            dia_corte: Some(32),
            dia_pago: None,
            origen: None,
        };
        assert!(t.validar().is_err());
    }
}
