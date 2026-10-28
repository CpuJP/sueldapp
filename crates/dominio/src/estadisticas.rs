//! Estadísticas para el tablero: indicadores de un rango de meses y agrupaciones de gasto.

use crate::calculos::TotalesMes;
use crate::{Dinero, Periodo};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Indicadores de un conjunto de meses.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Indicadores {
    pub meses: usize,
    pub ingreso: Dinero,
    pub gasto: Dinero,
    pub pagado: Dinero,
    /// Suma de restantes (ingreso − gasto).
    pub balance: Dinero,
    /// Suma de los restantes positivos.
    pub sobrante: Dinero,
    /// Suma de los restantes negativos, en positivo.
    pub faltante: Dinero,
    pub meses_con_sobrante: usize,
    pub meses_con_faltante: usize,
    pub promedio_ingreso: Dinero,
    pub promedio_gasto: Dinero,
    pub promedio_balance: Dinero,
    /// Balance / ingreso × 100, 1 decimal. `None` sin ingresos.
    pub tasa_ahorro: Option<Decimal>,
    pub mejor_mes: Option<(Periodo, Dinero)>,
    pub peor_mes: Option<(Periodo, Dinero)>,
    pub mes_mayor_gasto: Option<(Periodo, Dinero)>,
}

/// Porcentaje `parte / total × 100` con 1 decimal; `None` si el total es cero.
pub fn porcentaje(parte: Dinero, total: Dinero) -> Option<Decimal> {
    (!total.is_zero()).then(|| (parte * Decimal::ONE_HUNDRED / total).round_dp(1))
}

pub fn indicadores(meses: &[(Periodo, TotalesMes)]) -> Indicadores {
    let mut i = Indicadores {
        meses: meses.len(),
        ..Default::default()
    };
    for (p, t) in meses {
        i.ingreso += t.ingreso;
        i.gasto += t.total;
        i.pagado += t.pagado;
        i.balance += t.restante;
        if t.restante.is_sign_negative() && !t.restante.is_zero() {
            i.faltante -= t.restante;
            i.meses_con_faltante += 1;
        } else if !t.restante.is_zero() {
            i.sobrante += t.restante;
            i.meses_con_sobrante += 1;
        }
        if i.mejor_mes.is_none_or(|(_, b)| t.restante > b) {
            i.mejor_mes = Some((*p, t.restante));
        }
        if i.peor_mes.is_none_or(|(_, b)| t.restante < b) {
            i.peor_mes = Some((*p, t.restante));
        }
        if i.mes_mayor_gasto.is_none_or(|(_, g)| t.total > g) {
            i.mes_mayor_gasto = Some((*p, t.total));
        }
    }
    if !meses.is_empty() {
        let n = Decimal::from(meses.len());
        i.promedio_ingreso = (i.ingreso / n).round_dp(2);
        i.promedio_gasto = (i.gasto / n).round_dp(2);
        i.promedio_balance = (i.balance / n).round_dp(2);
    }
    i.tasa_ahorro = porcentaje(i.balance, i.ingreso);
    i
}

/// Saldo acumulado mes a mes (suma corrida de restantes).
pub fn acumulado(restantes: &[Dinero]) -> Vec<Dinero> {
    restantes
        .iter()
        .scan(Decimal::ZERO, |acc, r| {
            *acc += *r;
            Some(*acc)
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grupo {
    pub nombre: String,
    pub monto: Dinero,
    pub veces: usize,
    pub porcentaje: Option<Decimal>,
}

/// Suma por clave, de mayor a menor (empate: nombre). El nombre mostrado es el primero visto.
/// `clave(x)` devuelve (clave de agrupación, nombre visible).
pub fn agrupar<'a, T: 'a>(
    items: impl IntoIterator<Item = &'a T>,
    clave: impl Fn(&T) -> (String, String),
    monto: impl Fn(&T) -> Dinero,
) -> Vec<Grupo> {
    let mut mapa: HashMap<String, Grupo> = HashMap::new();
    let mut total = Decimal::ZERO;
    for x in items {
        let (k, nombre) = clave(x);
        let m = monto(x);
        total += m;
        let g = mapa.entry(k).or_insert(Grupo {
            nombre,
            monto: Decimal::ZERO,
            veces: 0,
            porcentaje: None,
        });
        g.monto += m;
        g.veces += 1;
    }
    let mut v: Vec<Grupo> = mapa.into_values().collect();
    for g in &mut v {
        g.porcentaje = porcentaje(g.monto, total);
    }
    v.sort_by(|a, b| b.monto.cmp(&a.monto).then_with(|| a.nombre.cmp(&b.nombre)));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn t(ingreso: Dinero, total: Dinero) -> TotalesMes {
        TotalesMes {
            ingreso,
            total,
            pagado: total,
            restante: ingreso - total,
            restante_pagado: ingreso - total,
        }
    }

    #[test]
    fn indicadores_basicos() {
        let meses = vec![
            ((2026, 1), t(dec!(1000), dec!(800))),
            ((2026, 2), t(dec!(1000), dec!(1300))),
            ((2026, 3), t(dec!(1000), dec!(900))),
        ];
        let i = indicadores(&meses);
        assert_eq!(i.balance, dec!(0));
        assert_eq!(i.sobrante, dec!(300));
        assert_eq!(i.faltante, dec!(300));
        assert_eq!((i.meses_con_sobrante, i.meses_con_faltante), (2, 1));
        assert_eq!(i.promedio_gasto, dec!(1000));
        assert_eq!(i.tasa_ahorro, Some(dec!(0.0)));
        assert_eq!(i.mejor_mes, Some(((2026, 1), dec!(200))));
        assert_eq!(i.peor_mes, Some(((2026, 2), dec!(-300))));
        assert_eq!(i.mes_mayor_gasto, Some(((2026, 2), dec!(1300))));
    }

    #[test]
    fn sin_meses() {
        let i = indicadores(&[]);
        assert_eq!((i.meses, i.tasa_ahorro, i.mejor_mes), (0, None, None));
    }

    #[test]
    fn acumula_y_agrupa() {
        assert_eq!(
            acumulado(&[dec!(10), dec!(-30), dec!(5)]),
            vec![dec!(10), dec!(-20), dec!(-15)]
        );
        let datos = [("Tigo", dec!(10)), ("tigo", dec!(30)), ("Claro", dec!(60))];
        let g = agrupar(
            datos.iter(),
            |x| (x.0.to_lowercase(), x.0.to_string()),
            |x| x.1,
        );
        assert_eq!(g[0].nombre, "Claro");
        assert_eq!(g[1].monto, dec!(40));
        assert_eq!(g[1].veces, 2);
        assert_eq!(g[1].porcentaje, Some(dec!(40.0)));
    }
}
