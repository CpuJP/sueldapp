//! Geometría de los gráficos: posiciones de pantalla en fracciones 0..1. Usa `f64` solo para
//! dibujar; los montos que se muestran vienen siempre del núcleo como texto exacto.
#![allow(clippy::float_arithmetic)]

use crate::{BarraMes, Eje};
use rust_decimal::prelude::ToPrimitive;
use slint::SharedString;
use sueldapp_dominio::Decimal;

pub fn f(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}

/// Escala "bonita": (mínimo, máximo, paso), siempre incluyendo el cero.
pub fn escala(min: f64, max: f64) -> (f64, f64, f64) {
    let (min, max) = (min.min(0.0), max.max(0.0));
    let (min, max) = if (max - min).abs() < f64::EPSILON {
        (min, min + 1.0)
    } else {
        (min, max)
    };
    let bruto = (max - min) / 4.0;
    let mag = 10f64.powf(bruto.log10().floor());
    let paso = [1.0, 2.0, 2.5, 5.0, 10.0]
        .iter()
        .map(|m| m * mag)
        .find(|p| *p >= bruto)
        .unwrap_or(bruto);
    (
        (min / paso).floor() * paso,
        (max / paso).ceil() * paso,
        paso,
    )
}

/// "$ 1,5 M", "$ 500 k", "-$ 2 M": rótulos compactos del eje.
pub fn eje_monto(v: f64) -> String {
    let signo = if v < 0.0 { "-" } else { "" };
    let a = v.abs();
    let (n, sufijo) = if a >= 1e6 {
        (a / 1e6, " M")
    } else if a >= 1e3 {
        (a / 1e3, " k")
    } else {
        (a, "")
    };
    let texto = if (n - n.round()).abs() < 0.05 {
        format!("{}", n.round())
    } else {
        format!("{:.1}", n).replace('.', ",")
    };
    format!("{signo}$ {texto}{sufijo}")
}

pub struct Escala {
    lo: f64,
    hi: f64,
    paso: f64,
}

impl Escala {
    pub fn nueva(valores: impl IntoIterator<Item = f64>) -> Escala {
        let v: Vec<f64> = valores.into_iter().collect();
        let min = v.iter().copied().fold(0.0, f64::min);
        let max = v.iter().copied().fold(0.0, f64::max);
        let (lo, hi, paso) = escala(min, max);
        Escala { lo, hi, paso }
    }

    /// Fracción desde arriba (0 = máximo, 1 = mínimo).
    pub fn y(&self, v: f64) -> f32 {
        ((self.hi - v) / (self.hi - self.lo)) as f32
    }

    pub fn ejes(&self) -> Vec<Eje> {
        let mut v = Vec::new();
        let mut x = self.lo;
        while x <= self.hi + self.paso / 2.0 {
            v.push(Eje {
                pos: self.y(x),
                texto: SharedString::from(eje_monto(x)),
            });
            x += self.paso;
        }
        v
    }
}

/// `v / max` acotado a 0..1 (0 si `max` es 0).
pub fn fraccion(v: f64, max: f64) -> f32 {
    if max <= 0.0 {
        0.0
    } else {
        (v / max).clamp(0.0, 1.0) as f32
    }
}

/// Cada cuántos meses poner rótulo en el eje X para que no se pisen.
pub fn rotulo(i: usize, n: usize, max_rotulos: usize) -> bool {
    let cada = n.div_ceil(max_rotulos.max(1)).max(1);
    i.is_multiple_of(cada)
}

/// Línea y área (viewbox 1000×1000) de una serie de fracciones.
pub fn trazos(barras: &[BarraMes]) -> (String, String) {
    if barras.is_empty() {
        return (String::new(), String::new());
    }
    let n = barras.len() as f64;
    let pts: Vec<(f64, f64)> = barras
        .iter()
        .enumerate()
        .map(|(i, b)| ((i as f64 + 0.5) / n * 1000.0, f64::from(b.a) * 1000.0))
        .collect();
    let base = f64::from(barras[0].base) * 1000.0;
    let linea = pts
        .iter()
        .enumerate()
        .map(|(i, (x, y))| format!("{}{x:.1} {y:.1}", if i == 0 { "M" } else { " L" }))
        .collect::<String>();
    let area = format!(
        "M{:.1} {base:.1} {} L{:.1} {base:.1} Z",
        pts[0].0,
        pts.iter()
            .map(|(x, y)| format!("L{x:.1} {y:.1}"))
            .collect::<Vec<_>>()
            .join(" "),
        pts[pts.len() - 1].0
    );
    (linea, area)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escalas_y_rotulos() {
        assert_eq!(escala(0.0, 3_700_000.0), (0.0, 4_000_000.0, 1_000_000.0));
        let (lo, hi, _) = escala(-253_000.0, 1_600_000.0);
        assert!(lo <= -253_000.0 && hi >= 1_600_000.0);
        assert_eq!(eje_monto(1_500_000.0), "$ 1,5 M");
        assert_eq!(eje_monto(-500_000.0), "-$ 500 k");
        assert_eq!(eje_monto(0.0), "$ 0");
        assert!(rotulo(0, 14, 7) && !rotulo(1, 14, 7) && rotulo(2, 14, 7));
    }
}
