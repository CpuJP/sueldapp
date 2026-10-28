//! Reglas de cálculo de la hoja: totales del mes, del año, saldos de deudas y tarjetas.

use crate::{Dinero, Gasto, Ingreso, Periodo, Tarjeta};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Valores de un mes, con los mismos nombres que la hoja.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TotalesMes {
    /// Ingreso del mes (sueldo + prima + otros).
    pub ingreso: Dinero,
    /// Total: todos los gastos del mes, sea cual sea su estado.
    pub total: Dinero,
    /// SubTotal: gastos cuyo estado cuenta como pagado.
    pub pagado: Dinero,
    /// Restante = ingreso − total.
    pub restante: Dinero,
    /// SubRestante = ingreso − pagado.
    pub restante_pagado: Dinero,
}

/// Calcula los totales de un mes. `es_pagado(estado_id)` dice si un estado cuenta como pagado.
/// Los gastos `fuera_de_bloque` no cuentan (igual que en la hoja).
pub fn totales_mes(
    gastos: &[Gasto],
    ingresos: &[Ingreso],
    es_pagado: impl Fn(i64) -> bool,
) -> TotalesMes {
    let validos = gastos.iter().filter(|g| !g.fuera_de_bloque);
    let mut total = Decimal::ZERO;
    let mut pagado = Decimal::ZERO;
    for g in validos {
        total += g.monto;
        if es_pagado(g.estado_id) {
            pagado += g.monto;
        }
    }
    let ingreso: Dinero = ingresos.iter().map(|i| i.monto).sum();
    TotalesMes {
        ingreso,
        total,
        pagado,
        restante: ingreso - total,
        restante_pagado: ingreso - pagado,
    }
}

/// Suma de varios meses (totales anuales).
pub fn sumar<'a>(meses: impl IntoIterator<Item = &'a TotalesMes>) -> TotalesMes {
    meses
        .into_iter()
        .fold(TotalesMes::default(), |acc, m| TotalesMes {
            ingreso: acc.ingreso + m.ingreso,
            total: acc.total + m.total,
            pagado: acc.pagado + m.pagado,
            restante: acc.restante + m.restante,
            restante_pagado: acc.restante_pagado + m.restante_pagado,
        })
}

/// Una cuota vinculada a una deuda.
#[derive(Debug, Clone, PartialEq)]
pub struct Cuota {
    pub monto: Dinero,
    pub pagada: bool,
    pub periodo: Periodo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaldoDeuda {
    /// Cuotas pagadas + ajuste.
    pub pagado: Dinero,
    /// Total − pagado. Negativo = pagado de más. `None` si la deuda no tiene total.
    pub pendiente: Option<Dinero>,
    /// Porcentaje 0–100 (tope 100). `None` sin total o con total cero.
    pub progreso: Option<Dinero>,
    pub cuotas_pagadas: usize,
    pub cuotas_registradas: usize,
    /// Periodo de la última cuota registrada.
    pub fin_estimado: Option<Periodo>,
}

pub fn saldo_deuda(total: Option<Dinero>, ajuste: Dinero, cuotas: &[Cuota]) -> SaldoDeuda {
    let pagadas: Vec<&Cuota> = cuotas.iter().filter(|c| c.pagada).collect();
    let pagado = ajuste + pagadas.iter().map(|c| c.monto).sum::<Dinero>();
    let pendiente = total.map(|t| t - pagado);
    let progreso = total.filter(|t| !t.is_zero()).map(|t| {
        let p = (pagado * Decimal::ONE_HUNDRED / t).round_dp(1);
        p.clamp(Decimal::ZERO, Decimal::ONE_HUNDRED)
    });
    SaldoDeuda {
        pagado,
        pendiente,
        progreso,
        cuotas_pagadas: pagadas.len(),
        cuotas_registradas: cuotas.len(),
        fin_estimado: cuotas.iter().map(|c| c.periodo).max(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaldoTarjeta {
    /// Cupo − saldo; `None` si falta alguno.
    pub disponible: Option<Dinero>,
    /// Suma de los pagos pagados.
    pub total_pagado: Dinero,
    pub pagos_registrados: usize,
}

pub fn saldo_tarjeta(t: &Tarjeta, pagos: &[Cuota]) -> SaldoTarjeta {
    SaldoTarjeta {
        disponible: t.cupo.zip(t.saldo).map(|(c, s)| c - s),
        total_pagado: pagos.iter().filter(|p| p.pagada).map(|p| p.monto).sum(),
        pagos_registrados: pagos.len(),
    }
}

/// Redondeo de valores importados: 2 decimales, punto medio lejos de cero.
pub fn redondear(d: Dinero) -> Dinero {
    d.round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const PAGADO: i64 = 1;
    const PENDIENTE: i64 = 2;

    fn g(monto: Dinero, estado_id: i64, fuera: bool) -> Gasto {
        Gasto {
            id: None,
            mes_id: 1,
            concepto: "x".into(),
            monto,
            estado_id,
            categoria_id: None,
            deuda_id: None,
            tarjeta_id: None,
            desglose: None,
            nota: String::new(),
            fuera_de_bloque: fuera,
            orden: 0,
            origen: None,
        }
    }

    fn i(monto: Dinero) -> Ingreso {
        Ingreso {
            id: None,
            mes_id: 1,
            fuente_id: 1,
            monto,
            desglose: None,
            nota: String::new(),
            origen: None,
        }
    }

    #[test]
    fn mes_como_la_hoja() {
        // Total = todo; Pagado = solo estados que cuentan como pagado; fuera de bloque no cuenta.
        let gastos = vec![
            g(dec!(1500000.25), PAGADO, false),
            g(dec!(400000), PENDIENTE, false),
            g(dec!(100000), PENDIENTE, false),
            g(dec!(250000), PAGADO, true),
        ];
        let t = totales_mes(&gastos, &[i(dec!(2500000))], |e| e == PAGADO);
        assert_eq!(t.total, dec!(2000000.25));
        assert_eq!(t.pagado, dec!(1500000.25));
        assert_eq!(t.restante, dec!(499999.75));
        assert_eq!(t.restante_pagado, dec!(999999.75));
    }

    #[test]
    fn mes_vacio_y_restante_negativo() {
        let t = totales_mes(&[], &[], |_| true);
        assert_eq!(t, TotalesMes::default());
        let t = totales_mes(&[g(dec!(10), PAGADO, false)], &[], |e| e == PAGADO);
        assert_eq!(t.restante, dec!(-10));
    }

    #[test]
    fn suma_anual() {
        let a = totales_mes(&[g(dec!(5), PAGADO, false)], &[i(dec!(10))], |_| true);
        let s = sumar([&a, &a]);
        assert_eq!(s.ingreso, dec!(20));
        assert_eq!(s.restante, dec!(10));
    }

    #[test]
    fn deuda_con_ajuste_y_pago_de_mas() {
        let c = |m, p, mes| Cuota {
            monto: m,
            pagada: p,
            periodo: (2026, mes),
        };
        let s = saldo_deuda(
            Some(dec!(800000)),
            dec!(0),
            &[c(dec!(410000), true, 1), c(dec!(410000), true, 2)],
        );
        assert_eq!(s.pendiente, Some(dec!(-20000)));
        assert_eq!(s.progreso, Some(dec!(100)));

        let s = saldo_deuda(
            Some(dec!(1000)),
            dec!(100),
            &[c(dec!(200), true, 3), c(dec!(200), false, 5)],
        );
        assert_eq!(s.pagado, dec!(300));
        assert_eq!(s.pendiente, Some(dec!(700)));
        assert_eq!(s.progreso, Some(dec!(30.0)));
        assert_eq!((s.cuotas_pagadas, s.cuotas_registradas), (1, 2));
        assert_eq!(s.fin_estimado, Some((2026, 5)));
    }

    #[test]
    fn deuda_sin_total() {
        let s = saldo_deuda(None, dec!(5), &[]);
        assert_eq!(
            (s.pendiente, s.progreso, s.fin_estimado),
            (None, None, None)
        );
        assert_eq!(s.pagado, dec!(5));
    }

    #[test]
    fn tarjeta_disponible() {
        let t = Tarjeta {
            id: None,
            nombre: "Tarjeta Azul".into(),
            cupo: Some(dec!(5000000)),
            saldo: Some(dec!(1200000.50)),
            dia_corte: None,
            dia_pago: None,
            origen: None,
        };
        let s = saldo_tarjeta(&t, &[]);
        assert_eq!(s.disponible, Some(dec!(3799999.50)));
    }

    #[test]
    fn redondeo() {
        assert_eq!(redondear(dec!(12345.3000000003)), dec!(12345.30));
        assert_eq!(redondear(dec!(-1.00999999977648)), dec!(-1.01));
        assert_eq!(redondear(dec!(0.005)), dec!(0.01));
    }
}
