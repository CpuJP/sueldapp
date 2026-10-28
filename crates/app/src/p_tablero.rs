//! Pantalla Tablero: indicadores, gráficos y desgloses del periodo elegido.

use crate::comun::{cifra, cifra_texto, modelo, s};
use crate::estado::{clave, de_clave, hoy, Estado, Rango};
use crate::geometria::{f, rotulo, trazos, Escala};
use crate::vistas::{self, GrupoVista, Monto};
use crate::{BarraMes, FilaTabla, Grupo, Linea, Preset, TableroD, Ventana};
use slint::ComponentHandle;
use sueldapp_almacen::repo;
use sueldapp_dominio::Periodo;

const CORTOS: [&str; 12] = [
    "Ene", "Feb", "Mar", "Abr", "May", "Jun", "Jul", "Ago", "Sep", "Oct", "Nov", "Dic",
];

fn texto_periodo(p: Periodo) -> String {
    format!("{} {}", CORTOS[p.1 as usize - 1], p.0)
}

/// Límites de los datos: primer y último mes con registros.
fn limites(est: &Estado) -> Result<(Periodo, Periodo), String> {
    let meses = repo::listar_meses(&est.c).map_err(|e| e.to_string())?;
    let h = hoy();
    let min = meses.first().map(|m| (m.anio, m.mes)).unwrap_or((h.0, 1));
    let max = meses.last().map(|m| (m.anio, m.mes)).unwrap_or((h.0, 12));
    Ok((min, max))
}

pub fn presets(min: Periodo, max: Periodo) -> Vec<(String, Periodo, Periodo)> {
    let h = hoy();
    let atras = |n: i32| de_clave(clave(h) - (n - 1));
    vec![
        ("Hasta hoy".into(), min, h),
        ("Este mes".into(), h, h),
        ("Últimos 3 meses".into(), atras(3), h),
        ("Últimos 6 meses".into(), atras(6), h),
        ("Últimos 12 meses".into(), atras(12), h),
        ("Este año".into(), (h.0, 1), (h.0, 12)),
        ("Año pasado".into(), (h.0 - 1, 1), (h.0 - 1, 12)),
        ("Todo (con proyección)".into(), min, max),
    ]
}

pub fn aplicar_preset(est: &mut Estado, i: usize) -> Result<(), String> {
    let (min, max) = limites(est)?;
    if let Some((n, d, h)) = presets(min, max).into_iter().nth(i) {
        est.rango = Some(Rango {
            desde: d,
            hasta: h,
            nombre: Some(n),
        });
    }
    Ok(())
}

fn grupos(v: &[GrupoVista], detalle: impl Fn(&GrupoVista) -> String) -> Vec<Grupo> {
    let max = v.iter().map(|g| f(g.monto.valor)).fold(0.0_f64, f64::max);
    v.iter()
        .map(|g| Grupo {
            nombre: s(&g.nombre),
            monto: s(&g.monto.texto),
            detalle: s(detalle(g)),
            frac: crate::geometria::fraccion(f(g.monto.valor), max),
        })
        .collect()
}

fn fila_tabla(c: Vec<String>, neg: Vec<bool>) -> FilaTabla {
    FilaTabla {
        c: modelo(c.into_iter().map(|x| s(&x)).collect()),
        negativos: modelo(neg),
    }
}

pub fn cargar(ui: &Ventana, est: &mut Estado) -> Result<(), String> {
    let (min, max) = limites(est)?;
    let rango = est.rango.get_or_insert(Rango {
        desde: min,
        hasta: hoy(),
        nombre: Some("Hasta hoy".into()),
    });
    let (desde, hasta) = (rango.desde, rango.hasta);
    let nombre = rango.nombre.clone();
    let t = vistas::tablero(&est.c, Some(desde), Some(hasta))?;
    let g = ui.global::<TableroD>();

    let lista = presets(min, max);
    g.set_presets(modelo(
        lista
            .iter()
            .map(|(n, d, h)| Preset {
                nombre: s(n),
                desde: clave(*d),
                hasta: clave(*h),
            })
            .collect(),
    ));
    g.set_preset_activo(
        lista
            .iter()
            .position(|(n, _, _)| Some(n) == nombre.as_ref())
            .map(|i| i as i32)
            .unwrap_or(-1),
    );
    let anios: Vec<i32> = repo::listar_meses(&est.c)
        .map_err(|e| e.to_string())?
        .iter()
        .map(|m| m.anio)
        .fold(Vec::new(), |mut v, a| {
            if !v.contains(&a) {
                v.push(a);
            }
            v
        });
    let es_anio = desde.1 == 1 && hasta.1 == 12 && desde.0 == hasta.0;
    g.set_anio_activo(if es_anio { desde.0 } else { -1 });
    g.set_anios(modelo(anios));
    g.set_rango_nombre(s(nombre
        .unwrap_or_else(|| "Periodo personalizado".into())
        .to_uppercase()));
    g.set_rango_texto(s(format!(
        "{} – {}",
        texto_periodo(desde),
        texto_periodo(hasta)
    )));
    g.set_desde(clave(desde));
    g.set_hasta(clave(hasta));
    g.set_datos_desde(clave(min));
    g.set_datos_hasta(clave(max));
    g.set_hoy(clave(hoy()));
    g.set_sin_meses(t.meses.is_empty());
    if t.meses.is_empty() {
        return Ok(());
    }

    let k = &t.kpis;
    g.set_hero_etiqueta(s(if k.balance.negativo {
        "TE FALTÓ EN EL PERIODO"
    } else {
        "TE SOBRÓ EN EL PERIODO"
    }));
    g.set_hero_valor(s(&k.balance.texto));
    g.set_hero_negativo(k.balance.negativo);
    g.set_hero_nota(s(format!(
        "{} – {} · {} meses.{}",
        t.desde.clone().unwrap_or_default(),
        t.hasta.clone().unwrap_or_default(),
        k.meses,
        k.tasa_ahorro
            .as_ref()
            .map(|p| format!(" Ahorraste el {} de lo que ingresó.", p.texto))
            .unwrap_or_default()
    )));
    g.set_cifras(modelo(vec![
        cifra("Ingresos", &k.ingreso, ""),
        cifra("Gastos", &k.gasto, ""),
        cifra_texto(
            "Meses con sobrante",
            &k.meses_con_sobrante.to_string(),
            &k.sobrante.texto,
            false,
        ),
        cifra_texto(
            "Meses con faltante",
            &k.meses_con_faltante.to_string(),
            &k.faltante.texto,
            k.meses_con_faltante > 0,
        ),
    ]));
    let mes_valor = |m: &Option<vistas::MesValor>, verbo: bool| -> (String, String) {
        match m {
            Some(m) => {
                let nota = if verbo {
                    format!(
                        "{} {}",
                        if m.monto.negativo { "Faltó" } else { "Sobró" },
                        m.monto.texto.replace('-', "")
                    )
                } else {
                    m.monto.texto.clone()
                };
                (m.nombre.clone(), nota)
            }
            None => ("—".into(), String::new()),
        }
    };
    let (mejor, mejor_n) = mes_valor(&k.mejor_mes, true);
    let (peor, peor_n) = mes_valor(&k.peor_mes, true);
    let (mayor, mayor_n) = mes_valor(&k.mes_mayor_gasto, false);
    g.set_kpis(modelo(vec![
        cifra("Ingreso promedio al mes", &k.promedio_ingreso, ""),
        cifra("Gasto promedio al mes", &k.promedio_gasto, ""),
        cifra("Sobra o falta en promedio", &k.promedio_balance, ""),
        cifra("Por pagar en el periodo", &k.por_pagar, ""),
        cifra_texto("Mejor mes", &mejor, &mejor_n, false),
        cifra_texto("Mes más ajustado", &peor, &peor_n, false),
        cifra_texto("Mes con más gasto", &mayor, &mayor_n, false),
        cifra(
            "Deuda pendiente total",
            &t.deuda_pendiente,
            &format!("Ya pagado en deudas: {}", t.deuda_pagada.texto),
        ),
    ]));

    // Gráficos.
    let n = t.meses.len();
    let tip = |m: &vistas::MesTablero, extra: String| format!("{}\n{extra}", m.nombre);
    let e1 = Escala::nueva(
        t.meses
            .iter()
            .flat_map(|m| [f(m.ingreso.valor), f(m.gasto.valor)]),
    );
    g.set_ejes_ig(modelo(e1.ejes()));
    g.set_ig(modelo(
        t.meses
            .iter()
            .enumerate()
            .map(|(i, m)| BarraMes {
                corto: s(&m.corto),
                a: e1.y(f(m.ingreso.valor)),
                b: e1.y(f(m.gasto.valor)),
                base: e1.y(0.0),
                negativo: false,
                rotulo: rotulo(i, n, 12),
                tip: s(tip(
                    m,
                    format!("Ingresos {}\nGastos {}", m.ingreso.texto, m.gasto.texto),
                )),
            })
            .collect(),
    ));
    let e2 = Escala::nueva(t.meses.iter().map(|m| f(m.balance.valor)));
    g.set_ejes_bal(modelo(e2.ejes()));
    g.set_bal(modelo(
        t.meses
            .iter()
            .enumerate()
            .map(|(i, m)| BarraMes {
                corto: s(&m.corto),
                a: e2.y(f(m.balance.valor)),
                b: 0.0,
                base: e2.y(0.0),
                negativo: m.balance.negativo,
                rotulo: rotulo(i, n, 6),
                tip: s(tip(
                    m,
                    format!(
                        "{} {}{}",
                        if m.balance.negativo {
                            "Faltó"
                        } else {
                            "Sobró"
                        },
                        m.balance.texto.replace('-', ""),
                        m.tasa_ahorro
                            .as_ref()
                            .map(|p| format!("\n{} del ingreso", p.texto))
                            .unwrap_or_default()
                    ),
                )),
            })
            .collect(),
    ));
    let e3 = Escala::nueva(t.meses.iter().map(|m| f(m.acumulado.valor)));
    let acu: Vec<BarraMes> = t
        .meses
        .iter()
        .enumerate()
        .map(|(i, m)| BarraMes {
            corto: s(&m.corto),
            a: e3.y(f(m.acumulado.valor)),
            b: 0.0,
            base: e3.y(0.0),
            negativo: m.acumulado.negativo,
            rotulo: rotulo(i, n, 6),
            tip: s(tip(m, format!("Acumulado {}", m.acumulado.texto))),
        })
        .collect();
    let (linea, area) = trazos(&acu);
    g.set_ejes_acu(modelo(e3.ejes()));
    g.set_acu(modelo(acu));
    g.set_acu_linea(s(linea));
    g.set_acu_area(s(area));

    let pct = |x: &GrupoVista| {
        x.porcentaje
            .as_ref()
            .map(|p| p.texto.clone())
            .unwrap_or_default()
    };
    g.set_destino(modelo(grupos(&t.por_destino, pct)));
    g.set_categoria(modelo(grupos(&t.por_categoria, pct)));
    g.set_estado(modelo(grupos(&t.por_estado, pct)));
    g.set_fuentes(modelo(grupos(&t.por_fuente, pct)));
    g.set_conceptos(modelo(grupos(&t.top_conceptos, |x| {
        format!("{} {}", x.veces, if x.veces == 1 { "vez" } else { "veces" })
    })));
    g.set_deudas(modelo(
        t.deudas
            .iter()
            .map(|d| Linea {
                titulo: s(&d.nombre),
                sub: s(d
                    .progreso
                    .map(|p| format!("{} % pagado", p.to_string().replace('.', ",")))
                    .unwrap_or_default()),
                monto: s(&d.pendiente.texto),
                negativo: false,
                progreso: d.progreso.map(f).unwrap_or(0.0) as f32,
                tiene_progreso: d.progreso.is_some(),
            })
            .collect(),
    ));
    g.set_mayores(modelo(
        t.mayores_gastos
            .iter()
            .map(|x| Linea {
                titulo: s(&x.gasto.concepto),
                sub: s(&x.periodo),
                monto: s(&x.monto_texto),
                negativo: false,
                progreso: 0.0,
                tiene_progreso: false,
            })
            .collect(),
    ));
    let neg = |m: &Monto| m.negativo;
    g.set_tabla(modelo(
        t.meses
            .iter()
            .map(|m| {
                fila_tabla(
                    vec![
                        m.nombre.clone(),
                        m.ingreso.texto.clone(),
                        m.gasto.texto.clone(),
                        m.pagado.texto.clone(),
                        m.balance.texto.clone(),
                        m.acumulado.texto.clone(),
                        m.tasa_ahorro
                            .as_ref()
                            .map(|p| p.texto.clone())
                            .unwrap_or("—".into()),
                    ],
                    vec![
                        false,
                        neg(&m.ingreso),
                        neg(&m.gasto),
                        false,
                        neg(&m.balance),
                        neg(&m.acumulado),
                        false,
                    ],
                )
            })
            .collect(),
    ));
    Ok(())
}
