//! Formato de montos en COP y lectura de montos escritos por el usuario.

use crate::{Dinero, Invalido};
use std::str::FromStr;

/// `$ 1.234.567,89`; negativos como `-$ 1,01` (igual que la hoja).
pub fn formatear(d: Dinero) -> String {
    let d = d.round_dp(2);
    let negativo = d.is_sign_negative() && !d.is_zero();
    let texto = format!("{:.2}", d.abs());
    let (entero, decimales) = texto.split_once('.').unwrap_or((&texto, "00"));
    let mut con_puntos = String::new();
    for (i, c) in entero.chars().enumerate() {
        if i > 0 && (entero.len() - i) % 3 == 0 {
            con_puntos.push('.');
        }
        con_puntos.push(c);
    }
    format!(
        "{}$ {},{}",
        if negativo { "-" } else { "" },
        con_puntos,
        decimales
    )
}

/// Lee un monto como lo escribiría una persona en Colombia.
///
/// - Con coma: la coma es decimal y los puntos son de miles (`1.234,5`).
/// - Sin coma: un punto seguido de exactamente 3 cifras, o varios puntos, son de miles
///   (`1.234`, `1.234.567`); si no, el punto es decimal (`1234.5`).
pub fn leer_monto(texto: &str) -> Result<Dinero, Invalido> {
    let error = || {
        Invalido(format!(
            "\"{}\" no es un monto válido. Escribe solo números, por ejemplo 150.000 o 150000,50.",
            texto.trim()
        ))
    };
    let limpio: String = texto
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '$')
        .collect();
    if limpio.is_empty() {
        return Err(Invalido("Escribe un monto.".into()));
    }
    let normal = if limpio.contains(',') {
        limpio.replace('.', "").replacen(',', ".", 1)
    } else {
        let puntos = limpio.matches('.').count();
        let tras_punto = limpio.rsplit('.').next().unwrap_or("");
        if puntos > 1 || (puntos == 1 && tras_punto.len() == 3) {
            limpio.replace('.', "")
        } else {
            limpio
        }
    };
    if !normal
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    {
        return Err(error());
    }
    let d = Dinero::from_str(&normal).map_err(|_| error())?;
    if d.scale() > 2 && d != d.round_dp(2) {
        return Err(Invalido("Usa como máximo 2 decimales.".into()));
    }
    Ok(d.round_dp(2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn formatea_como_la_hoja() {
        assert_eq!(formatear(dec!(1234567)), "$ 1.234.567,00");
        assert_eq!(formatear(dec!(-1.01)), "-$ 1,01");
        assert_eq!(formatear(dec!(0)), "$ 0,00");
        assert_eq!(formatear(dec!(999.5)), "$ 999,50");
        assert_eq!(formatear(dec!(-0)), "$ 0,00");
    }

    #[test]
    fn lee_montos() {
        assert_eq!(leer_monto("150.000").unwrap(), dec!(150000));
        assert_eq!(leer_monto("$ 1.234.567,89").unwrap(), dec!(1234567.89));
        assert_eq!(leer_monto("150000,5").unwrap(), dec!(150000.5));
        assert_eq!(leer_monto("1234.5").unwrap(), dec!(1234.5));
        assert_eq!(leer_monto(" 12500 ").unwrap(), dec!(12500));
    }

    #[test]
    fn rechaza_basura() {
        assert!(leer_monto("").is_err());
        assert!(leer_monto("abc").is_err());
        assert!(leer_monto("12,345,6").is_err());
        assert!(leer_monto("1,234").is_err()); // coma decimal con 3 decimales
        assert!(leer_monto("1,2345").is_err());
    }
}
