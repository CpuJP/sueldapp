//! Formato de montos en COP y lectura de montos escritos por el usuario.

use crate::{Dinero, Invalido};
use std::str::FromStr;

fn fecha_existe(anio: u32, mes: u32, dia: u32) -> bool {
    let bisiesto =
        anio.is_multiple_of(4) && (!anio.is_multiple_of(100) || anio.is_multiple_of(400));
    let dias = match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if bisiesto => 29,
        2 => 28,
        _ => return false,
    };
    (1000..=9999).contains(&anio) && (1..=dias).contains(&dia)
}

/// Lee `día/mes/año` (también con `-`) y la devuelve como `AAAA-MM-DD`; vacío es `None`.
pub fn leer_fecha(texto: &str) -> Result<Option<String>, Invalido> {
    let texto = texto.trim();
    if texto.is_empty() {
        return Ok(None);
    }
    let partes: Vec<Option<u32>> = texto
        .split(['/', '-'])
        .map(|p| p.trim().parse().ok())
        .collect();
    match partes[..] {
        [Some(d), Some(m), Some(a)] if fecha_existe(a, m, d) => {
            Ok(Some(format!("{a:04}-{m:02}-{d:02}")))
        }
        _ => Err(Invalido(format!(
            "«{texto}» no es una fecha válida. Escríbela como día/mes/año, por ejemplo 23/11/2026."
        ))),
    }
}

/// `true` si `iso` es una fecha real con la forma `AAAA-MM-DD`.
pub fn fecha_valida(iso: &str) -> bool {
    let p: Vec<&str> = iso.split('-').collect();
    match p[..] {
        [a, m, d] if a.len() == 4 && m.len() == 2 && d.len() == 2 => {
            match (a.parse(), m.parse(), d.parse()) {
                (Ok(a), Ok(m), Ok(d)) => fecha_existe(a, m, d),
                _ => false,
            }
        }
        _ => false,
    }
}

/// Días desde `desde` hasta `hasta` (negativo si `hasta` ya pasó); `None` si alguna no es válida.
pub fn dias_entre(desde: &str, hasta: &str) -> Option<i64> {
    // Algoritmo civil de Howard Hinnant.
    let dia_absoluto = |iso: &str| -> Option<i64> {
        if !fecha_valida(iso) {
            return None;
        }
        let mut p = iso.split('-').map(|x| x.parse::<i64>().ok());
        let (a, m, d) = (p.next()??, p.next()??, p.next()??);
        let y = if m <= 2 { a - 1 } else { a };
        let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
        Some(
            y.div_euclid(400) * 146_097 + y.rem_euclid(400) * 365 + y.rem_euclid(400) / 4
                - y.rem_euclid(400) / 100
                + doy,
        )
    };
    Some(dia_absoluto(hasta)? - dia_absoluto(desde)?)
}

/// `AAAA-MM-DD` como `DD/MM/AAAA`.
pub fn formatear_fecha(iso: &str) -> String {
    let p: Vec<&str> = iso.split('-').collect();
    match p[..] {
        [a, m, d] => format!("{d}/{m}/{a}"),
        _ => iso.to_string(),
    }
}

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
    fn cuenta_dias_entre_fechas() {
        assert_eq!(dias_entre("2026-11-02", "2026-11-23"), Some(21));
        assert_eq!(dias_entre("2026-11-23", "2026-11-23"), Some(0));
        assert_eq!(dias_entre("2026-11-24", "2026-11-23"), Some(-1));
        assert_eq!(dias_entre("2026-12-31", "2027-01-01"), Some(1));
        assert_eq!(dias_entre("2028-02-28", "2028-03-01"), Some(2));
        assert_eq!(dias_entre("2026-02-28", "2026-03-01"), Some(1));
        assert_eq!(dias_entre("2000-01-01", "2026-01-01"), Some(9497));
        assert_eq!(dias_entre("mal", "2026-01-01"), None);
    }

    #[test]
    fn lee_y_formatea_fechas() {
        assert_eq!(leer_fecha("23/11/2026").unwrap().unwrap(), "2026-11-23");
        assert_eq!(leer_fecha(" 3-2-2028 ").unwrap().unwrap(), "2028-02-03");
        assert_eq!(leer_fecha("29/02/2028").unwrap().unwrap(), "2028-02-29");
        assert_eq!(leer_fecha("  ").unwrap(), None);
        for mala in [
            "29/02/2026",
            "31/04/2026",
            "23/13/2026",
            "23/11",
            "23/11/26",
            "ayer",
        ] {
            assert!(leer_fecha(mala).is_err(), "{mala}");
        }
        assert!(fecha_valida("2026-11-23"));
        assert!(!fecha_valida("2026-02-30"));
        assert!(!fecha_valida("2026-1-5"));
        assert_eq!(formatear_fecha("2026-11-23"), "23/11/2026");
    }

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
