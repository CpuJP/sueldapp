//! Conciliación: compara lo que calcula la app sobre los datos importados con los valores que
//! calculaba la hoja. Cualquier diferencia no aceptada hace fallar la importación.

use crate::hoja::Nomina;
use crate::Error;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use sueldapp_almacen::repo;
use sueldapp_dominio::calculos::{saldo_deuda, sumar, totales_mes, Cuota, TotalesMes};
use sueldapp_dominio::{Decimal, Periodo, NOMBRES_MES};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diferencia {
    pub concepto: String,
    pub hoja: Option<Decimal>,
    pub app: Option<Decimal>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Resultado {
    /// Número de valores comparados.
    pub comparados: usize,
    pub diferencias: Vec<Diferencia>,
    /// Diferencias aprobadas por el usuario (ver spec, Clarifications).
    pub aceptadas: Vec<Diferencia>,
}

impl Resultado {
    fn comparar(
        &mut self,
        concepto: String,
        hoja: Option<Decimal>,
        app: Option<Decimal>,
        aceptada: bool,
    ) {
        let Some(h) = hoja else {
            return;
        };
        self.comparados += 1;
        if Some(h) != app {
            let d = Diferencia {
                concepto,
                hoja,
                app,
            };
            if aceptada {
                self.aceptadas.push(d);
            } else {
                self.diferencias.push(d);
            }
        }
    }
}

fn nombre_mes(m: u32) -> &'static str {
    NOMBRES_MES.get(m as usize - 1).copied().unwrap_or("?")
}

/// `mes_ids[i]` es el id del mes del bloque `i`; `deuda_ids[i]` el de la deuda `i`;
/// `deuda_aceptada[i]` si sus diferencias están aprobadas. Solo cuentan registros cuyo
/// `origen` empieza por `prefijo` (los importados de la hoja).
pub fn conciliar(
    c: &Connection,
    n: &Nomina,
    prefijo: &str,
    mes_ids: &[i64],
    deuda_ids: &[i64],
    deuda_aceptada: &[bool],
) -> Result<Resultado, Error> {
    let estados = repo::listar_estados(c)?;
    let es_pagado = |id: i64| estados.iter().any(|e| e.id == Some(id) && e.cuenta_pagado);
    let de_hoja = |o: &Option<String>| o.as_deref().is_some_and(|o| o.starts_with(prefijo));
    let mut r = Resultado::default();

    let mut por_anio: BTreeMap<i32, Vec<TotalesMes>> = BTreeMap::new();
    for (b, &mes_id) in n.bloques.iter().zip(mes_ids) {
        let gastos: Vec<_> = repo::gastos_de_mes(c, mes_id)?
            .into_iter()
            .filter(|g| de_hoja(&g.origen))
            .collect();
        let ingresos: Vec<_> = repo::ingresos_de_mes(c, mes_id)?
            .into_iter()
            .filter(|i| de_hoja(&i.origen))
            .collect();
        let t = totales_mes(&gastos, &ingresos, es_pagado);
        let m = format!("{} {}", nombre_mes(b.mes), b.anio);
        r.comparar(format!("Total {m}"), b.total, Some(t.total), false);
        r.comparar(format!("SubTotal {m}"), b.subtotal, Some(t.pagado), false);
        r.comparar(
            format!("Sueldo {m}"),
            b.sueldo.as_ref().map(|v| v.valor),
            Some(t.ingreso),
            false,
        );
        r.comparar(format!("Restante {m}"), b.restante, Some(t.restante), false);
        r.comparar(
            format!("SubRestante {m}"),
            b.subrestante,
            Some(t.restante_pagado),
            false,
        );
        por_anio.entry(b.anio).or_default().push(t);
    }
    for a in &n.anios {
        let t = sumar(por_anio.get(&a.anio).map(|v| v.as_slice()).unwrap_or(&[]));
        r.comparar(
            format!("TOTAL PAGO {}", a.anio),
            a.pago,
            Some(t.ingreso),
            false,
        );
        r.comparar(
            format!("TOTAL GASTO {}", a.anio),
            a.gasto,
            Some(t.total),
            false,
        );
        r.comparar(
            format!("TOTAL RESTANTE {}", a.anio),
            a.restante,
            Some(t.restante),
            false,
        );
    }

    let periodos: HashMap<i64, Periodo> = repo::listar_meses(c)?
        .into_iter()
        .filter_map(|m| Some((m.id?, (m.anio, m.mes))))
        .collect();
    let gastos = repo::todos_gastos(c)?;
    for ((d, &id), &aceptada) in n.deudas.iter().zip(deuda_ids).zip(deuda_aceptada) {
        let deuda = repo::deuda(c, id)?;
        let cuotas: Vec<Cuota> = gastos
            .iter()
            .filter(|g| g.deuda_id == Some(id))
            .map(|g| Cuota {
                monto: g.monto,
                pagada: es_pagado(g.estado_id),
                periodo: periodos.get(&g.mes_id).copied().unwrap_or((0, 0)),
            })
            .collect();
        let s = saldo_deuda(deuda.total, deuda.ajuste, &cuotas);
        let nombre = &d.nombre;
        r.comparar(
            format!("Pago deuda {nombre}"),
            d.pago.as_ref().map(|v| v.valor),
            Some(s.pagado),
            aceptada,
        );
        r.comparar(
            format!("Total deuda {nombre}"),
            d.total.as_ref().map(|v| v.valor),
            deuda.total,
            aceptada,
        );
        r.comparar(
            format!("Pendiente deuda {nombre}"),
            d.pendiente,
            s.pendiente,
            aceptada,
        );
        if aceptada {
            r.aceptadas.push(Diferencia {
                concepto: format!("Total deuda {nombre} (calculado de sus cuotas)"),
                hoja: None,
                app: deuda.total,
            });
        }
    }
    Ok(r)
}
