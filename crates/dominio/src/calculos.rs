//! Reglas de cálculo de la hoja: totales del mes, del año, saldos de deudas y tarjetas.

use crate::{Dinero, Gasto, Ingreso, Periodo, Tarjeta};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// En qué punto está el pago de un mes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SituacionMes {
    SinGastos,
    /// Número de pagos que faltan; un mes así nunca cuenta como hecho.
    Pendientes(usize),
    /// Todo pagado, a la espera de que la persona lo confirme.
    PorConfirmar,
    Hecho,
}

/// "No" y "Aplazado" son decisiones ya tomadas: no se espera ese pago en este mes.
pub fn estado_espera_pago(nombre: &str, cuenta_pagado: bool) -> bool {
    !cuenta_pagado && !matches!(crate::texto::normalizar(nombre).as_str(), "no" | "aplazado")
}

/// Los gastos fuera de bloque no cuentan, igual que en los totales del mes.
pub fn situacion_mes(
    cerrado: bool,
    gastos: &[Gasto],
    espera_pago: impl Fn(i64) -> bool,
) -> SituacionMes {
    let cuentan = gastos.iter().filter(|g| !g.fuera_de_bloque);
    let pendientes = cuentan.clone().filter(|g| espera_pago(g.estado_id)).count();
    if pendientes > 0 {
        SituacionMes::Pendientes(pendientes)
    } else if cerrado {
        SituacionMes::Hecho
    } else if cuentan.count() == 0 {
        SituacionMes::SinGastos
    } else {
        SituacionMes::PorConfirmar
    }
}

/// Situación de un mes con lo que falta por pagar.
#[derive(Debug, Clone, PartialEq)]
pub struct EstadoMes {
    pub periodo: Periodo,
    pub situacion: SituacionMes,
    /// Pagos que faltan y cuánto suman.
    pub por_pagar: Dinero,
    /// Pagos de un mes ya pasado cuya fecha de pago, si tiene, también pasó.
    pub pagos_atrasados: usize,
    pub por_pagar_atrasado: Dinero,
}

/// Qué falta en un mes. Con `hoy` en `AAAA-MM-DD`, un pago con fecha futura no está atrasado.
pub fn estado_mes(
    periodo: Periodo,
    cerrado: bool,
    gastos: &[Gasto],
    espera_pago: impl Fn(i64) -> bool,
    hoy: &str,
) -> EstadoMes {
    let situacion = situacion_mes(cerrado, gastos, &espera_pago);
    let pendientes = gastos
        .iter()
        .filter(|g| !g.fuera_de_bloque && espera_pago(g.estado_id));
    let ya_paso =
        format!("{:04}-{:02}", periodo.0, periodo.1).as_str() < hoy.get(..7).unwrap_or("");
    let atrasados = pendientes
        .clone()
        .filter(|g| ya_paso && g.fecha_pago.as_deref().is_none_or(|f| f < hoy));
    EstadoMes {
        periodo,
        situacion,
        por_pagar: pendientes.map(|g| g.monto).sum(),
        pagos_atrasados: atrasados.clone().count(),
        por_pagar_atrasado: atrasados.map(|g| g.monto).sum(),
    }
}

/// Cifras de un conjunto de meses. Los meses sin gastos no cuentan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResumenEstados {
    pub con_gastos: usize,
    pub hechos: usize,
    pub por_confirmar: usize,
    pub pendientes: usize,
    pub atrasados: usize,
    pub pagos_pendientes: usize,
    pub pagos_atrasados: usize,
    pub por_pagar: Dinero,
    pub por_pagar_atrasado: Dinero,
    /// Mes más antiguo que aún tiene pagos pendientes.
    pub pendiente_mas_antiguo: Option<Periodo>,
}

pub fn resumir_estados(meses: &[EstadoMes]) -> ResumenEstados {
    let mut r = ResumenEstados::default();
    for m in meses {
        match m.situacion {
            SituacionMes::SinGastos => continue,
            SituacionMes::Hecho => r.hechos += 1,
            SituacionMes::PorConfirmar => r.por_confirmar += 1,
            SituacionMes::Pendientes(n) => {
                r.pendientes += 1;
                r.pagos_pendientes += n;
                r.pendiente_mas_antiguo = r
                    .pendiente_mas_antiguo
                    .map_or(Some(m.periodo), |p| Some(p.min(m.periodo)));
            }
        }
        r.con_gastos += 1;
        r.atrasados += usize::from(m.pagos_atrasados > 0);
        r.pagos_atrasados += m.pagos_atrasados;
        r.por_pagar += m.por_pagar;
        r.por_pagar_atrasado += m.por_pagar_atrasado;
    }
    r
}

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
    #[test]
    fn estado_y_resumen_de_meses() {
        use rust_decimal_macros::dec;
        let espera = |id: i64| id == 2;
        let hoy = "2026-11-02";
        let pago = |monto, estado, fecha: Option<&str>| {
            let mut x = g(monto, estado, false);
            x.fecha_pago = fecha.map(String::from);
            x
        };
        // Mes pasado: sin fecha o con fecha pasada está atrasado; con fecha futura, no.
        let octubre = estado_mes(
            (2026, 10),
            false,
            &[
                pago(dec!(10), 2, None),
                pago(dec!(20), 2, Some("2026-11-01")),
                pago(dec!(40), 2, Some("2026-11-23")),
                pago(dec!(80), 1, None),
                g(dec!(160), 2, true),
            ],
            espera,
            hoy,
        );
        assert_eq!(octubre.situacion, SituacionMes::Pendientes(3));
        assert_eq!(octubre.por_pagar, dec!(70));
        assert_eq!(octubre.pagos_atrasados, 2);
        assert_eq!(octubre.por_pagar_atrasado, dec!(30));
        // El mes actual y los futuros nunca están atrasados.
        let actual = estado_mes((2026, 11), false, &[pago(dec!(5), 2, None)], espera, hoy);
        assert_eq!(actual.pagos_atrasados, 0);
        let hecho = estado_mes((2026, 9), true, &[pago(dec!(5), 1, None)], espera, hoy);
        let por_confirmar = estado_mes((2026, 8), false, &[pago(dec!(5), 1, None)], espera, hoy);
        let vacio = estado_mes((2026, 7), false, &[], espera, hoy);
        assert_eq!(vacio.situacion, SituacionMes::SinGastos);

        let r = resumir_estados(&[actual, octubre, hecho, por_confirmar, vacio]);
        assert_eq!(r.con_gastos, 4);
        assert_eq!(r.hechos, 1);
        assert_eq!(r.por_confirmar, 1);
        assert_eq!(r.pendientes, 2);
        assert_eq!(r.atrasados, 1);
        assert_eq!(r.pagos_pendientes, 4);
        assert_eq!(r.pagos_atrasados, 2);
        assert_eq!(r.por_pagar, dec!(75));
        assert_eq!(r.por_pagar_atrasado, dec!(30));
        assert_eq!(r.pendiente_mas_antiguo, Some((2026, 10)));
        assert_eq!(resumir_estados(&[]), ResumenEstados::default());
    }

    #[test]
    fn situacion_del_mes() {
        let espera = |id: i64| id == 2;
        let pagado = g(rust_decimal_macros::dec!(1), 1, false);
        let pendiente = g(rust_decimal_macros::dec!(1), 2, false);
        let fuera = g(rust_decimal_macros::dec!(1), 2, true);
        use SituacionMes::*;
        assert_eq!(situacion_mes(false, &[], espera), SinGastos);
        assert_eq!(situacion_mes(true, &[], espera), Hecho);
        let solo_fuera = [fuera.clone()];
        assert_eq!(situacion_mes(false, &solo_fuera, espera), SinGastos);
        let todo = [pagado.clone(), fuera];
        assert_eq!(situacion_mes(false, &todo, espera), PorConfirmar);
        assert_eq!(situacion_mes(true, &todo, espera), Hecho);
        let falta = [pagado, pendiente];
        assert_eq!(situacion_mes(false, &falta, espera), Pendientes(1));
        assert_eq!(situacion_mes(true, &falta, espera), Pendientes(1));
    }

    #[test]
    fn que_estados_esperan_pago() {
        assert!(estado_espera_pago("Pendiente", false));
        assert!(estado_espera_pago("Sin estado", false));
        assert!(!estado_espera_pago("Pagado", true));
        assert!(!estado_espera_pago("NO", false));
        assert!(!estado_espera_pago("Aplazado", false));
    }

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
            fecha_pago: None,
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
