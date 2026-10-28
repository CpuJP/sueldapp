//! Carga de datos de cada pantalla (salvo el tablero) en los globals de la interfaz.

use crate::comun::{cifra, fila, gastos_paginados, modelo, pagina_vacia, paginar_local, s, textos};
use crate::estado::Estado;
use crate::vistas::{self, Filtro};
use crate::{
    App, CatsD, DatosD, DeudaFila, DeudasD, FilaTabla, ItemFila, Linea, LineaLog, MovsD, ResumenD,
    TarjetaFila, TarjetasD, Ventana,
};
use rust_decimal::prelude::ToPrimitive;
use slint::ComponentHandle;
use sueldapp_almacen::repo::{self, Catalogo};
use sueldapp_dominio::NOMBRES_MES;

type R = Result<(), String>;

fn e(x: impl std::fmt::Display) -> String {
    x.to_string()
}

fn f32_de(d: sueldapp_dominio::Decimal) -> f32 {
    d.to_f32().unwrap_or(0.0)
}

fn fila_tabla(c: Vec<String>, neg: Vec<bool>) -> FilaTabla {
    FilaTabla {
        c: textos(c),
        negativos: modelo(neg),
    }
}

pub fn general(ui: &Ventana, est: &Estado) -> R {
    let app = ui.global::<App>();
    app.set_vacia(repo::listar_meses(&est.c).map_err(e)?.is_empty());
    app.set_tam_pagina(est.tam_indice);
    Ok(())
}

// ---------- Resumen ----------

pub fn resumen(ui: &Ventana, est: &mut Estado) -> R {
    let g = ui.global::<ResumenD>();
    let (anio, mes) = est.resumen;
    g.set_modo(est.modo_resumen);
    if est.modo_resumen == 1 {
        let r = vistas::resumen_anio(&est.c, anio)?;
        let t = &r.totales;
        g.set_periodo(s(anio.to_string()));
        g.set_existe(true);
        g.set_hero_etiqueta(s(format!("TE QUEDA EN {anio}")));
        g.set_hero_valor(s(&t.restante.texto));
        g.set_hero_negativo(t.restante.negativo);
        g.set_hero_nota(s("Suma de los restantes de cada mes."));
        g.set_cifras(modelo(vec![
            cifra("Ingresos del año", &t.ingreso, ""),
            cifra("Gastos del año", &t.total, ""),
            cifra("Ya pagado", &t.pagado, ""),
            cifra("Por pagar", &t.pendiente, ""),
        ]));
        let mut filas: Vec<FilaTabla> = r
            .meses
            .iter()
            .map(|m| {
                fila_tabla(
                    vec![
                        if m.existe {
                            m.nombre.clone()
                        } else {
                            format!("{} (sin datos)", m.nombre)
                        },
                        m.totales.ingreso.texto.clone(),
                        m.totales.total.texto.clone(),
                        m.totales.pagado.texto.clone(),
                        m.totales.restante.texto.clone(),
                    ],
                    vec![false, false, false, false, m.totales.restante.negativo],
                )
            })
            .collect();
        filas.push(fila_tabla(
            vec![
                "Total".into(),
                t.ingreso.texto.clone(),
                t.total.texto.clone(),
                t.pagado.texto.clone(),
                t.restante.texto.clone(),
            ],
            vec![false, false, false, false, t.restante.negativo],
        ));
        g.set_anio(modelo(filas));
        return Ok(());
    }
    let r = vistas::resumen_mes(&est.c, anio, mes)?;
    let t = &r.totales;
    g.set_periodo(s(&r.nombre));
    g.set_existe(r.mes.is_some());
    g.set_cerrado(r.mes.as_ref().is_some_and(|m| m.cerrado));
    g.set_hero_etiqueta(s("TE QUEDA ESTE MES"));
    g.set_hero_valor(s(&t.restante.texto));
    g.set_hero_negativo(t.restante.negativo);
    g.set_hero_nota(s(format!(
        "{} De lo ya pagado te queda {}.",
        if t.restante.negativo {
            "Los gastos del mes superan tus ingresos."
        } else {
            "Ingresos menos todos los gastos del mes."
        },
        t.restante_pagado.texto
    )));
    g.set_cifras(modelo(vec![
        cifra("Ingresos", &t.ingreso, ""),
        cifra("Gastos del mes", &t.total, ""),
        cifra("Ya pagado", &t.pagado, ""),
        cifra("Por pagar", &t.pendiente, ""),
    ]));
    let avance = sueldapp_dominio::estadisticas::porcentaje(t.pagado.valor, t.total.valor)
        .unwrap_or_default()
        .round();
    g.set_avance(f32_de(avance));
    g.set_avance_texto(s(format!(
        "Has pagado el {avance} % de los gastos del mes."
    )));

    let clave = format!("{anio}-{mes}");
    let proximos: Vec<_> = r.proximos.iter().map(fila).collect();
    for p in &r.proximos {
        if let Some(id) = p.gasto.id {
            est.visibles.insert(id, p.gasto.clone());
        }
    }
    let (trozo, pag) = paginar_local(est, &format!("proximos-{clave}"), &proximos);
    g.set_proximos(modelo(trozo));
    g.set_pag_proximos(pag);

    for i in &r.ingresos {
        if let Some(id) = i.ingreso.id {
            est.ingresos_visibles.insert(id, i.ingreso.clone());
        }
    }
    let ingresos: Vec<(i32, Linea)> = r
        .ingresos
        .iter()
        .map(|i| {
            (
                i.ingreso.id.unwrap_or(0) as i32,
                Linea {
                    titulo: s(&i.fuente),
                    sub: s(i
                        .ingreso
                        .desglose
                        .as_ref()
                        .map(|d| format!("Desglose {d}"))
                        .unwrap_or_default()),
                    monto: s(&i.monto_texto),
                    negativo: false,
                    progreso: 0.0,
                    tiene_progreso: false,
                },
            )
        })
        .collect();
    let (trozo, pag) = paginar_local(est, &format!("ingresos-{clave}"), &ingresos);
    g.set_ingresos_ids(modelo(trozo.iter().map(|x| x.0).collect()));
    g.set_ingresos(modelo(trozo.into_iter().map(|x| x.1).collect()));
    g.set_pag_ingresos(pag);

    let (filas, pag, _) = gastos_paginados(
        est,
        &format!("gastos-{clave}"),
        Filtro {
            anio: Some(anio),
            mes: Some(mes),
            ..Default::default()
        },
    )?;
    g.set_gastos(modelo(filas));
    g.set_pag_gastos(pag);
    g.set_total(s(&t.total.texto));
    Ok(())
}

// ---------- Movimientos ----------

pub struct OpcionesMovs {
    pub anios: Vec<i32>,
    pub estados: Vec<i64>,
    pub categorias: Vec<i64>,
    pub deudas: Vec<i64>,
    pub tarjetas: Vec<i64>,
}

pub fn opciones_movs(est: &Estado) -> Result<OpcionesMovs, String> {
    let c = vistas::catalogos(&est.c)?;
    let mut anios: Vec<i32> = repo::listar_meses(&est.c)
        .map_err(e)?
        .iter()
        .map(|m| m.anio)
        .collect();
    anios.dedup();
    Ok(OpcionesMovs {
        anios,
        estados: c.estados.iter().filter_map(|x| x.id).collect(),
        categorias: c.categorias.iter().filter_map(|x| x.id).collect(),
        deudas: c.deudas.iter().filter_map(|x| x.id).collect(),
        tarjetas: c.tarjetas.iter().filter_map(|x| x.id).collect(),
    })
}

pub fn movimientos(ui: &Ventana, est: &mut Estado) -> R {
    let g = ui.global::<MovsD>();
    let c = vistas::catalogos(&est.c)?;
    let o = opciones_movs(est)?;
    let con_todos = |todos: &str, v: Vec<String>| {
        let mut x = vec![todos.to_string()];
        x.extend(v);
        textos(x)
    };
    g.set_o_anios(con_todos(
        "Todos",
        o.anios.iter().map(|a| a.to_string()).collect(),
    ));
    g.set_o_meses(con_todos(
        "Todos",
        NOMBRES_MES.iter().map(|m| m.to_string()).collect(),
    ));
    g.set_o_estados(con_todos(
        "Todos",
        c.estados.iter().map(|x| x.nombre.clone()).collect(),
    ));
    g.set_o_pagos(textos(["Todos", "Solo pagados", "Solo sin pagar"]));
    g.set_o_categorias(con_todos(
        "Todas",
        c.categorias.iter().map(|x| x.nombre.clone()).collect(),
    ));
    g.set_o_deudas(con_todos(
        "Todas",
        c.deudas.iter().map(|x| x.nombre.clone()).collect(),
    ));
    g.set_o_tarjetas(con_todos(
        "Todas",
        c.tarjetas.iter().map(|x| x.nombre.clone()).collect(),
    ));
    let elegido = |i: i32, v: &Vec<i64>| {
        if i <= 0 {
            None
        } else {
            v.get(i as usize - 1).copied()
        }
    };
    let filtro = Filtro {
        texto: Some(g.get_texto().to_string()).filter(|t| !t.trim().is_empty()),
        anio: if g.get_anio() <= 0 {
            None
        } else {
            o.anios.get(g.get_anio() as usize - 1).copied()
        },
        mes: if g.get_mes() <= 0 {
            None
        } else {
            Some(g.get_mes() as u32)
        },
        estado_id: elegido(g.get_estado(), &o.estados),
        categoria_id: elegido(g.get_categoria(), &o.categorias),
        deuda_id: elegido(g.get_deuda(), &o.deudas),
        tarjeta_id: elegido(g.get_tarjeta(), &o.tarjetas),
        pago: match g.get_pago() {
            1 => Some("pagados".into()),
            2 => Some("pendientes".into()),
            _ => None,
        },
        ..Default::default()
    };
    let (filas, pag, r) = gastos_paginados(est, "movimientos", filtro)?;
    g.set_titulo(s(format!("{} movimientos", r.cantidad)));
    g.set_suma(s(&r.total.texto));
    g.set_filas(modelo(filas));
    g.set_pagina(pag);
    Ok(())
}

// ---------- Deudas y tarjetas ----------

pub fn deudas(ui: &Ventana, est: &mut Estado) -> R {
    let g = ui.global::<DeudasD>();
    let lista = vistas::listar_deudas(&est.c)?;
    let mut filas = Vec::new();
    for d in &lista {
        let id = d.deuda.id.unwrap_or(0);
        let abierta = est.abiertas.contains(&id);
        let (cuotas, pag) = if abierta {
            let (f, p, _) = gastos_paginados(
                est,
                &format!("cuotas-{id}"),
                Filtro {
                    deuda_id: Some(id),
                    ..Default::default()
                },
            )?;
            (f, p)
        } else {
            (vec![], pagina_vacia())
        };
        let pendiente = match &d.pendiente {
            Some(p) if p.negativo => format!("Pagado de más: {}", p.texto.replace('-', "")),
            Some(p) => format!("Pendiente: {}", p.texto),
            None => "Sin total definido".into(),
        };
        filas.push((
            d.deuda.saldada,
            DeudaFila {
                id: id as i32,
                nombre: s(&d.deuda.nombre),
                saldada: d.deuda.saldada,
                progreso: d.progreso.map(f32_de).unwrap_or(0.0),
                tiene_progreso: d.progreso.is_some(),
                progreso_texto: s(d
                    .progreso
                    .map(|p| format!("{} % pagado", p.to_string().replace('.', ",")))
                    .unwrap_or_default()),
                pagado: s(&d.pagado.texto),
                total: s(d.total_texto.clone().unwrap_or_default()),
                pendiente: s(pendiente),
                cuotas: s(format!("{} de {}", d.cuotas_pagadas, d.cuotas_registradas)),
                fin: s(d.fin_estimado.clone().unwrap_or("—".into())),
                abierta,
                num: d.cuotas.len() as i32,
                filas: modelo(cuotas),
                pagina: pag,
            },
        ));
    }
    let activas: Vec<DeudaFila> = filas.iter().filter(|x| !x.0).map(|x| x.1.clone()).collect();
    let saldadas: Vec<DeudaFila> = filas.iter().filter(|x| x.0).map(|x| x.1.clone()).collect();
    g.set_num_activas(activas.len() as i32);
    g.set_num_saldadas(saldadas.len() as i32);
    let (a, pa) = paginar_local(est, "deudas-activas", &activas);
    let (b, pb) = paginar_local(est, "deudas-saldadas", &saldadas);
    g.set_activas(modelo(a));
    g.set_pag_activas(pa);
    g.set_saldadas(modelo(b));
    g.set_pag_saldadas(pb);
    Ok(())
}

pub fn tarjetas(ui: &Ventana, est: &mut Estado) -> R {
    let g = ui.global::<TarjetasD>();
    let lista = vistas::listar_tarjetas(&est.c)?;
    let mut filas = Vec::new();
    for t in &lista {
        let id = t.tarjeta.id.unwrap_or(0);
        let abierta = est.abiertas_tarjetas.contains(&id);
        let (pagos, pag) = if abierta {
            let (f, p, _) = gastos_paginados(
                est,
                &format!("pagos-{id}"),
                Filtro {
                    tarjeta_id: Some(id),
                    ..Default::default()
                },
            )?;
            (f, p)
        } else {
            (vec![], pagina_vacia())
        };
        let uso = t
            .tarjeta
            .cupo
            .zip(t.tarjeta.saldo)
            .and_then(|(c, sd)| sueldapp_dominio::estadisticas::porcentaje(sd, c))
            .map(|p| p.round().min(sueldapp_dominio::Decimal::ONE_HUNDRED));
        filas.push(TarjetaFila {
            id: id as i32,
            nombre: s(&t.tarjeta.nombre),
            registrados: s(format!("{} pagos registrados", t.pagos_registrados)),
            uso: uso.map(f32_de).unwrap_or(0.0),
            tiene_uso: uso.is_some(),
            uso_texto: s(uso
                .map(|u| format!("{u} % del cupo usado"))
                .unwrap_or_default()),
            cupo: s(t.cupo_texto.clone().unwrap_or("—".into())),
            saldo: s(t.saldo_texto.clone().unwrap_or("—".into())),
            disponible: s(t
                .disponible
                .as_ref()
                .map(|d| d.texto.clone())
                .unwrap_or("—".into())),
            disponible_negativo: t.disponible.as_ref().is_some_and(|d| d.negativo),
            corte: s(t
                .tarjeta
                .dia_corte
                .map(|d| format!("día {d}"))
                .unwrap_or("—".into())),
            pago: s(t
                .tarjeta
                .dia_pago
                .map(|d| format!("día {d}"))
                .unwrap_or("—".into())),
            total_pagado: s(&t.total_pagado.texto),
            abierta,
            num: t.pagos.len() as i32,
            filas: modelo(pagos),
            pagina: pag,
        });
    }
    let (trozo, pag) = paginar_local(est, "tarjetas", &filas);
    g.set_tarjetas(modelo(trozo));
    g.set_pagina(pag);
    Ok(())
}

// ---------- Catálogos ----------

pub fn catalogos(ui: &Ventana, est: &mut Estado) -> R {
    let g = ui.global::<CatsD>();
    let c = vistas::catalogos(&est.c)?;
    let estados: Vec<ItemFila> = c
        .estados
        .iter()
        .map(|x| ItemFila {
            id: x.id.unwrap_or(0) as i32,
            nombre: s(&x.nombre),
            sub: s(if x.cuenta_pagado {
                "Cuenta como pagado"
            } else {
                "No cuenta como pagado"
            }),
        })
        .collect();
    let items = |v: &[sueldapp_dominio::Item]| -> Vec<ItemFila> {
        v.iter()
            .map(|x| ItemFila {
                id: x.id.unwrap_or(0) as i32,
                nombre: s(&x.nombre),
                sub: s(""),
            })
            .collect()
    };
    let (a, pa) = paginar_local(est, "cat-0", &estados);
    let (b, pb) = paginar_local(est, "cat-1", &items(&c.categorias));
    let (d, pd) = paginar_local(est, "cat-2", &items(&c.fuentes));
    g.set_estados(modelo(a));
    g.set_pag_estados(pa);
    g.set_categorias(modelo(b));
    g.set_pag_categorias(pb);
    g.set_fuentes(modelo(d));
    g.set_pag_fuentes(pd);
    Ok(())
}

pub fn catalogo_de(tipo: i32) -> Catalogo {
    if tipo == 2 {
        Catalogo::Fuentes
    } else {
        Catalogo::Categorias
    }
}

// ---------- Importar y exportar ----------

pub fn datos(ui: &Ventana, est: &mut Estado) -> R {
    let g = ui.global::<DatosD>();
    let historial: Vec<Linea> = repo::listar_importaciones(&est.c)
        .map_err(e)?
        .iter()
        .map(|h| Linea {
            titulo: s(format!(
                "{} · {}",
                h.archivo,
                if h.ok { "Correcta" } else { "Fallida" }
            )),
            sub: s(format!("{} · {}", h.fecha, h.resumen)),
            monto: s(""),
            negativo: !h.ok,
            progreso: 0.0,
            tiene_progreso: false,
        })
        .collect();
    let (trozo, pag) = paginar_local(est, "historial", &historial);
    g.set_historial(modelo(trozo));
    g.set_pag_historial(pag);
    match &est.ultimo_informe {
        None => g.set_hay_informe(false),
        Some(inf) => {
            g.set_hay_informe(true);
            g.set_informe_ok(inf.ok);
            g.set_informe_titulo(s(if inf.ya_importado {
                "Este archivo ya estaba importado: no se cambió nada."
            } else if inf.ok {
                "Importación correcta: los totales coinciden."
            } else {
                "Importación fallida: hay diferencias y no se guardó nada."
            }));
            let cc = &inf.conciliacion;
            let n = |x: usize| x.to_string();
            g.set_informe_datos(modelo(vec![
                crate::comun::cifra_texto("Archivo", &inf.archivo, &inf.formato, false),
                crate::comun::cifra_texto("Meses", &n(inf.conteos.meses), "", false),
                crate::comun::cifra_texto("Gastos", &n(inf.conteos.gastos), "", false),
                crate::comun::cifra_texto("Ingresos", &n(inf.conteos.ingresos), "", false),
                crate::comun::cifra_texto("Deudas", &n(inf.conteos.deudas), "", false),
                crate::comun::cifra_texto("Tarjetas", &n(inf.conteos.tarjetas), "", false),
                crate::comun::cifra_texto("Valores comparados", &n(cc.comparados), "", false),
                crate::comun::cifra_texto(
                    "Diferencias",
                    &n(cc.diferencias.len()),
                    "",
                    !cc.diferencias.is_empty(),
                ),
            ]));
            let dif = |v: &Vec<sueldapp_ods::conciliar::Diferencia>| {
                modelo(
                    v.iter()
                        .map(|d| {
                            fila_tabla(
                                vec![
                                    d.concepto.clone(),
                                    d.hoja.map(|x| x.to_string()).unwrap_or("—".into()),
                                    d.app.map(|x| x.to_string()).unwrap_or("—".into()),
                                ],
                                vec![],
                            )
                        })
                        .collect(),
                )
            };
            g.set_diferencias(dif(&cc.diferencias));
            g.set_aceptadas(dif(&cc.aceptadas));
            let cuenta = |a: sueldapp_ods::importar::Accion| {
                inf.log.iter().filter(|l| l.accion == a).count()
            };
            use sueldapp_ods::importar::Accion as A;
            g.set_log_titulo(s(format!(
                "Registro detallado: {} importadas, {} transformadas, {} rechazadas, {} omitidas",
                cuenta(A::Importada),
                cuenta(A::Transformada),
                cuenta(A::Rechazada),
                cuenta(A::Omitida)
            )));
            g.set_log(modelo(
                inf.log
                    .iter()
                    .map(|l| LineaLog {
                        origen: s(&l.origen),
                        accion: s(format!("{:?}", l.accion)),
                        detalle: s(&l.detalle),
                    })
                    .collect(),
            ));
        }
    }
    Ok(())
}
