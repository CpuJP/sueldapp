//! Fórmulas simples de la hoja: sumas y restas de referencias y constantes
//! (`=+[.G29]+[.G45]+0.5`, `=1000+[.L108]`). Cualquier otra cosa devuelve `None`.

use std::str::FromStr;
use sueldapp_dominio::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub enum Termino {
    /// (fila, columna) desde 0.
    Ref(u32, u32),
    Num(Decimal),
}

/// Sumandos con su signo (`true` = resta).
pub fn sumandos(formula: &str) -> Option<Vec<(bool, Termino)>> {
    let f = formula.strip_prefix("of:").unwrap_or(formula);
    let f = f.strip_prefix('=')?;
    let b = f.as_bytes();
    let mut i = 0;
    let mut negativo = false;
    let mut hay_signo = true;
    let mut v = Vec::new();
    while i < b.len() {
        match b[i] {
            b' ' => i += 1,
            b'+' => {
                hay_signo = true;
                i += 1;
            }
            b'-' => {
                negativo = !negativo;
                hay_signo = true;
                i += 1;
            }
            b'[' if hay_signo => {
                let fin = f[i..].find(']')? + i;
                v.push((negativo, referencia(&f[i + 1..fin])?));
                i = fin + 1;
                negativo = false;
                hay_signo = false;
            }
            c if c.is_ascii_digit() && hay_signo => {
                let ini = i;
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                    i += 1;
                }
                v.push((negativo, Termino::Num(Decimal::from_str(&f[ini..i]).ok()?)));
                negativo = false;
                hay_signo = false;
            }
            _ => return None,
        }
    }
    if hay_signo || v.is_empty() {
        return None;
    }
    Some(v)
}

/// `.L163` → (162, 11). Solo referencias a la misma hoja.
fn referencia(r: &str) -> Option<Termino> {
    let r = r.strip_prefix('.')?;
    let letras: String = r.chars().take_while(|c| c.is_ascii_uppercase()).collect();
    let fila: u32 = r[letras.len()..].parse().ok()?;
    if letras.is_empty() || fila == 0 {
        return None;
    }
    let col = letras
        .bytes()
        .fold(0u32, |acc, c| acc * 26 + u32::from(c - b'A' + 1))
        - 1;
    Some(Termino::Ref(fila - 1, col))
}

/// Suma de las constantes (con su signo) y lista de referencias sumadas.
pub fn separar(terminos: &[(bool, Termino)]) -> (Decimal, Vec<(u32, u32)>) {
    let mut constante = Decimal::ZERO;
    let mut refs = Vec::new();
    for (neg, t) in terminos {
        match t {
            Termino::Num(n) if *neg => constante -= *n,
            Termino::Num(n) => constante += *n,
            Termino::Ref(f, c) => refs.push((*f, *c)),
        }
    }
    (constante, refs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn suma_de_referencias_y_ajuste() {
        let t = sumandos("=+[.G29]+[.L146]+0.5").unwrap();
        assert_eq!(
            t,
            vec![
                (false, Termino::Ref(28, 6)),
                (false, Termino::Ref(145, 11)),
                (false, Termino::Num(dec!(0.5)))
            ]
        );
        let (c, r) = separar(&t);
        assert_eq!(c, dec!(0.5));
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn constantes_y_restas() {
        let t = sumandos("of:=1000+250-50").unwrap();
        assert_eq!(separar(&t).0, dec!(1200));
        let t = sumandos("=1000+[.L108]").unwrap();
        assert_eq!(t[1], (false, Termino::Ref(107, 11)));
        assert_eq!(sumandos("=+[.AH35]").unwrap()[0].1, Termino::Ref(34, 33));
    }

    #[test]
    fn rechaza_lo_demas() {
        assert!(sumandos("=250000*6").is_none());
        assert!(sumandos("=+SUM([.G9:.G19])").is_none());
        assert!(sumandos("=5+").is_none());
        assert!(sumandos("5").is_none());
    }
}
