//! Acciones de la interfaz: navegación, formularios, borrar con deshacer, importar/exportar.

use crate::comun::{modelo, s, textos};
use crate::estado::{guardar_ajustes, AccionConfirmada, Ajustes, Deshacer, Estado, Formulario};
use crate::vistas::{self, DeudaEntrada, GastoEntrada, IngresoEntrada, TarjetaEntrada};
use crate::{p_tablero, pantallas, App, Aviso, Confirmar, Form, Tema, Ventana};
use slint::{ComponentHandle, Weak};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use sueldapp_almacen::repo;
use sueldapp_dominio::formato::formatear;
use sueldapp_dominio::{Decimal, Estado as EstadoGasto, Item, NOMBRES_MES};

pub type Est = Rc<RefCell<Estado>>;

/// Monto para un campo editable: "950.000,00" (sin el signo $).
fn monto_campo(d: Decimal) -> String {
    formatear(d).replace("$ ", "")
}

fn opcional(v: &str) -> Option<String> {
    Some(v.trim().to_string()).filter(|x| !x.is_empty())
}

// ---------- Avisos y recarga ----------

pub fn aviso(ui: &Ventana, est: &Est, texto: &str, error: bool, deshacer: Option<Deshacer>) {
    let id = {
        let mut e = est.borrow_mut();
        let id = e.sig_aviso;
        e.sig_aviso += 1;
        let con_deshacer = deshacer.is_some();
        if let Some(d) = deshacer {
            e.deshacer.insert(id, d);
        }
        e.avisos.push(Aviso {
            id,
            texto: s(texto),
            error,
            deshacer: con_deshacer,
        });
        ui.global::<App>().set_avisos(modelo(e.avisos.clone()));
        id
    };
    let (debil, est2) = (ui.as_weak(), est.clone());
    let espera = if est.borrow().deshacer.contains_key(&id) {
        9
    } else {
        5
    };
    slint::Timer::single_shot(Duration::from_secs(espera), move || {
        if let Some(ui) = debil.upgrade() {
            quitar_aviso(&ui, &est2, id);
        }
    });
}

pub fn quitar_aviso(ui: &Ventana, est: &Est, id: i32) {
    let mut e = est.borrow_mut();
    e.avisos.retain(|a| a.id != id);
    e.deshacer.remove(&id);
    ui.global::<App>().set_avisos(modelo(e.avisos.clone()));
}

pub fn recargar(ui: &Ventana, est: &Est) {
    let vista = ui.global::<App>().get_vista();
    let r = {
        let mut e = est.borrow_mut();
        pantallas::general(ui, &e).and_then(|_| match vista {
            0 => p_tablero::cargar(ui, &mut e),
            1 => pantallas::resumen(ui, &mut e),
            2 => pantallas::movimientos(ui, &mut e),
            3 => pantallas::deudas(ui, &mut e),
            4 => pantallas::tarjetas(ui, &mut e),
            5 => pantallas::catalogos(ui, &mut e),
            6 => pantallas::datos(ui, &mut e),
            _ => Ok(()),
        })
    };
    if let Err(m) = r {
        aviso(ui, est, &m, true, None);
    }
}

pub fn navegar(ui: &Ventana, est: &Est, vista: i32) {
    ui.global::<App>().set_vista(vista.clamp(0, 7));
    recargar(ui, est);
}

pub fn ajustes_cambiados(ui: &Ventana, est: &Est) {
    let t = ui.global::<Tema>();
    guardar_ajustes(&Ajustes {
        tema: t.get_modo(),
        solido: t.get_solido(),
        sin_movimiento: t.get_sin_movimiento(),
        tam: est.borrow().tam_indice,
    });
}

pub fn cambiar_tam(ui: &Ventana, est: &Est, i: i32) {
    {
        let mut e = est.borrow_mut();
        e.tam_indice = i.clamp(0, 2);
        e.cursores.clear();
        e.inicios.clear();
    }
    ajustes_cambiados(ui, est);
    recargar(ui, est);
}

// ---------- Formularios ----------

fn abrir_form(ui: &Ventana, tipo: i32, titulo: &str, puede_borrar: bool) {
    let f = ui.global::<Form>();
    f.set_error(s(""));
    f.set_titulo(s(titulo));
    f.set_puede_borrar(puede_borrar);
    f.set_tipo(tipo);
}

pub fn cerrar_form(ui: &Ventana, est: &Est) {
    ui.global::<Form>().set_tipo(0);
    est.borrow_mut().form = None;
}

/// Abre el formulario de gasto: existente (`id`) o nuevo con valores base.
pub fn abrir_gasto(
    ui: &Ventana,
    est: &Est,
    id: Option<i64>,
    deuda: Option<i64>,
    tarjeta: Option<i64>,
) {
    let r = (|| -> Result<(), String> {
        let mut e = est.borrow_mut();
        let cat = vistas::catalogos(&e.c)?;
        let g = match id {
            Some(id) => Some(match e.visibles.get(&id) {
                Some(g) => g.clone(),
                None => repo::gasto(&e.c, id).map_err(|x| x.to_string())?,
            }),
            None => None,
        };
        let periodo = match &g {
            Some(g) => {
                let m = repo::mes(&e.c, g.mes_id).map_err(|x| x.to_string())?;
                (m.anio, m.mes)
            }
            None => e.resumen,
        };
        let estados: Vec<i64> = cat.estados.iter().filter_map(|x| x.id).collect();
        let mut categorias = vec![None];
        categorias.extend(cat.categorias.iter().map(|x| x.id));
        let mut deudas = vec![None];
        deudas.extend(cat.deudas.iter().map(|x| x.id));
        let mut tarjetas = vec![None];
        tarjetas.extend(cat.tarjetas.iter().map(|x| x.id));
        let f = ui.global::<Form>();
        let con = |primero: &str, v: Vec<String>| {
            let mut x = vec![primero.to_string()];
            x.extend(v);
            textos(x)
        };
        f.set_o1(textos(cat.estados.iter().map(|x| x.nombre.clone())));
        f.set_o2(textos(NOMBRES_MES));
        f.set_o3(con(
            "Sin categoría",
            cat.categorias.iter().map(|x| x.nombre.clone()).collect(),
        ));
        f.set_o4(con(
            "Ninguna",
            cat.deudas.iter().map(|x| x.nombre.clone()).collect(),
        ));
        f.set_o5(con(
            "Ninguna",
            cat.tarjetas.iter().map(|x| x.nombre.clone()).collect(),
        ));
        let pendiente = cat
            .estados
            .iter()
            .position(|x| x.nombre == "Pendiente")
            .unwrap_or(0);
        let pos = |v: &Vec<Option<i64>>, x: Option<i64>| {
            v.iter().position(|y| *y == x).unwrap_or(0) as i32
        };
        f.set_t1(s(g
            .as_ref()
            .map(|g| g.concepto.clone())
            .unwrap_or_default()));
        f.set_t2(s(g
            .as_ref()
            .map(|g| monto_campo(g.monto))
            .unwrap_or_default()));
        f.set_t3(s(periodo.0.to_string()));
        f.set_t4(s(g.as_ref().map(|g| g.nota.clone()).unwrap_or_default()));
        f.set_s1(
            g.as_ref()
                .and_then(|g| estados.iter().position(|x| *x == g.estado_id))
                .unwrap_or(pendiente) as i32,
        );
        f.set_s2(periodo.1 as i32 - 1);
        f.set_s3(pos(&categorias, g.as_ref().and_then(|g| g.categoria_id)));
        f.set_s4(pos(
            &deudas,
            g.as_ref().map(|g| g.deuda_id).unwrap_or(deuda),
        ));
        f.set_s5(pos(
            &tarjetas,
            g.as_ref().map(|g| g.tarjeta_id).unwrap_or(tarjeta),
        ));
        f.set_b1(g.as_ref().is_some_and(|g| g.fuera_de_bloque));
        e.form = Some(Formulario::Gasto {
            id,
            estados,
            categorias,
            deudas,
            tarjetas,
        });
        Ok(())
    })();
    match r {
        Ok(()) => abrir_form(
            ui,
            1,
            if id.is_some() {
                "Editar gasto"
            } else {
                "Nuevo gasto"
            },
            id.is_some(),
        ),
        Err(m) => aviso(ui, est, &m, true, None),
    }
}

pub fn abrir_ingreso(ui: &Ventana, est: &Est, id: Option<i64>) {
    let r = (|| -> Result<(), String> {
        let mut e = est.borrow_mut();
        let cat = vistas::catalogos(&e.c)?;
        let i = id.and_then(|id| e.ingresos_visibles.get(&id).cloned());
        let fuentes: Vec<i64> = cat.fuentes.iter().filter_map(|x| x.id).collect();
        let f = ui.global::<Form>();
        f.set_o1(textos(cat.fuentes.iter().map(|x| x.nombre.clone())));
        f.set_o2(textos(NOMBRES_MES));
        f.set_s1(
            i.as_ref()
                .and_then(|i| fuentes.iter().position(|x| *x == i.fuente_id))
                .unwrap_or(0) as i32,
        );
        f.set_t2(s(i
            .as_ref()
            .map(|i| monto_campo(i.monto))
            .unwrap_or_default()));
        f.set_s2(e.resumen.1 as i32 - 1);
        f.set_t3(s(e.resumen.0.to_string()));
        f.set_t4(s(i.as_ref().map(|i| i.nota.clone()).unwrap_or_default()));
        e.form = Some(Formulario::Ingreso { id, fuentes });
        Ok(())
    })();
    match r {
        Ok(()) => abrir_form(
            ui,
            2,
            if id.is_some() {
                "Editar ingreso"
            } else {
                "Nuevo ingreso"
            },
            id.is_some(),
        ),
        Err(m) => aviso(ui, est, &m, true, None),
    }
}

pub fn abrir_deuda(ui: &Ventana, est: &Est, id: Option<i64>) {
    let r = (|| -> Result<(), String> {
        let mut e = est.borrow_mut();
        let d = id
            .map(|id| repo::deuda(&e.c, id))
            .transpose()
            .map_err(|x| x.to_string())?;
        let f = ui.global::<Form>();
        f.set_t1(s(d.as_ref().map(|d| d.nombre.clone()).unwrap_or_default()));
        f.set_t2(s(d
            .as_ref()
            .and_then(|d| d.total)
            .map(monto_campo)
            .unwrap_or_default()));
        f.set_t3(s(d
            .as_ref()
            .map(|d| monto_campo(d.ajuste))
            .unwrap_or("0".into())));
        f.set_t4(s(d.as_ref().map(|d| d.notas.clone()).unwrap_or_default()));
        f.set_b1(d.as_ref().is_some_and(|d| d.saldada));
        e.form = Some(Formulario::Deuda { id });
        Ok(())
    })();
    match r {
        Ok(()) => abrir_form(
            ui,
            3,
            if id.is_some() {
                "Editar deuda"
            } else {
                "Nueva deuda"
            },
            id.is_some(),
        ),
        Err(m) => aviso(ui, est, &m, true, None),
    }
}

pub fn abrir_tarjeta(ui: &Ventana, est: &Est, id: Option<i64>) {
    let r = (|| -> Result<(), String> {
        let mut e = est.borrow_mut();
        let t = id
            .map(|id| repo::tarjeta(&e.c, id))
            .transpose()
            .map_err(|x| x.to_string())?;
        let f = ui.global::<Form>();
        f.set_t1(s(t.as_ref().map(|t| t.nombre.clone()).unwrap_or_default()));
        f.set_t2(s(t
            .as_ref()
            .and_then(|t| t.cupo)
            .map(monto_campo)
            .unwrap_or_default()));
        f.set_t3(s(t
            .as_ref()
            .and_then(|t| t.saldo)
            .map(monto_campo)
            .unwrap_or_default()));
        f.set_t4(s(t
            .as_ref()
            .and_then(|t| t.dia_corte)
            .map(|d| d.to_string())
            .unwrap_or_default()));
        f.set_t5(s(t
            .as_ref()
            .and_then(|t| t.dia_pago)
            .map(|d| d.to_string())
            .unwrap_or_default()));
        e.form = Some(Formulario::Tarjeta { id });
        Ok(())
    })();
    match r {
        Ok(()) => abrir_form(
            ui,
            4,
            if id.is_some() {
                "Editar tarjeta"
            } else {
                "Nueva tarjeta"
            },
            id.is_some(),
        ),
        Err(m) => aviso(ui, est, &m, true, None),
    }
}

pub fn abrir_mes(ui: &Ventana, est: &Est) {
    let r = (|| -> Result<String, String> {
        let mut e = est.borrow_mut();
        let (a, m) = e.resumen;
        let mes = repo::mes_por_periodo(&e.c, a, m)
            .map_err(|x| x.to_string())?
            .ok_or("Este mes aún no tiene datos.")?;
        let nombre = format!("{} {a}", NOMBRES_MES[m as usize - 1]);
        ui.global::<Form>().set_b1(mes.cerrado);
        e.form = Some(Formulario::Mes {
            mes,
            nombre: nombre.clone(),
        });
        Ok(nombre)
    })();
    match r {
        Ok(n) => abrir_form(ui, 5, &format!("Editar {n}"), true),
        Err(m) => aviso(ui, est, &m, true, None),
    }
}

pub fn abrir_item(ui: &Ventana, est: &Est, tipo: i32, id: i64) {
    let r = (|| -> Result<String, String> {
        let mut e = est.borrow_mut();
        let cat = vistas::catalogos(&e.c)?;
        let (nombre, cuenta, orden, otros): (String, bool, i64, Vec<(Option<i64>, String)>) =
            if tipo == 0 {
                let x = cat
                    .estados
                    .iter()
                    .find(|x| x.id == Some(id))
                    .ok_or("No se encontró.")?;
                let otros = cat
                    .estados
                    .iter()
                    .filter(|y| y.id != Some(id))
                    .map(|y| (y.id, y.nombre.clone()))
                    .collect();
                (x.nombre.clone(), x.cuenta_pagado, x.orden, otros)
            } else {
                let lista = if tipo == 1 {
                    &cat.categorias
                } else {
                    &cat.fuentes
                };
                let x = lista
                    .iter()
                    .find(|x| x.id == Some(id))
                    .ok_or("No se encontró.")?;
                let otros = lista
                    .iter()
                    .filter(|y| y.id != Some(id))
                    .map(|y| (y.id, y.nombre.clone()))
                    .collect();
                (x.nombre.clone(), false, 0, otros)
            };
        let mut reasignar: Vec<Option<i64>> = vec![None];
        reasignar.extend(otros.iter().map(|o| o.0));
        let mut nombres = vec![if tipo == 1 {
            "Sin categoría".to_string()
        } else {
            "Elige…".to_string()
        }];
        nombres.extend(otros.iter().map(|o| o.1.clone()));
        let f = ui.global::<Form>();
        f.set_t1(s(&nombre));
        f.set_b1(cuenta);
        f.set_es_estado(tipo == 0);
        f.set_o1(textos(nombres));
        f.set_s1(0);
        e.form = Some(Formulario::Item {
            tipo,
            id,
            orden,
            reasignar,
        });
        Ok(nombre)
    })();
    match r {
        Ok(n) => abrir_form(ui, 6, &format!("Editar {n}"), true),
        Err(m) => aviso(ui, est, &m, true, None),
    }
}

/// Guarda el formulario abierto. Devuelve el texto del aviso o el error para el formulario.
pub fn guardar(ui: &Ventana, est: &Est) {
    let f = ui.global::<Form>();
    let r = (|| -> Result<String, String> {
        let e = est.borrow();
        match e.form.as_ref().ok_or("No hay formulario abierto.")? {
            Formulario::Gasto {
                id,
                estados,
                categorias,
                deudas,
                tarjetas,
            } => {
                let anio: i32 = f
                    .get_t3()
                    .trim()
                    .parse()
                    .map_err(|_| "El año debe ser un número de 4 cifras.")?;
                vistas::guardar_gasto(
                    &e.c,
                    GastoEntrada {
                        id: *id,
                        anio,
                        mes: f.get_s2() as u32 + 1,
                        concepto: f.get_t1().to_string(),
                        monto: f.get_t2().to_string(),
                        estado_id: *estados.get(f.get_s1() as usize).ok_or("Elige un estado.")?,
                        categoria_id: categorias.get(f.get_s3() as usize).copied().flatten(),
                        deuda_id: deudas.get(f.get_s4() as usize).copied().flatten(),
                        tarjeta_id: tarjetas.get(f.get_s5() as usize).copied().flatten(),
                        nota: f.get_t4().to_string(),
                        fuera_de_bloque: f.get_b1(),
                    },
                )?;
                Ok("Gasto guardado.".into())
            }
            Formulario::Ingreso { id, fuentes } => {
                let anio: i32 = f
                    .get_t3()
                    .trim()
                    .parse()
                    .map_err(|_| "El año debe ser un número de 4 cifras.")?;
                vistas::guardar_ingreso(
                    &e.c,
                    IngresoEntrada {
                        id: *id,
                        anio,
                        mes: f.get_s2() as u32 + 1,
                        fuente_id: *fuentes
                            .get(f.get_s1() as usize)
                            .ok_or("Elige una fuente.")?,
                        monto: f.get_t2().to_string(),
                        nota: f.get_t4().to_string(),
                    },
                )?;
                Ok("Ingreso guardado.".into())
            }
            Formulario::Deuda { id } => {
                vistas::guardar_deuda(
                    &e.c,
                    DeudaEntrada {
                        id: *id,
                        nombre: f.get_t1().to_string(),
                        total: opcional(&f.get_t2()),
                        ajuste: opcional(&f.get_t3()),
                        saldada: f.get_b1(),
                        notas: f.get_t4().to_string(),
                    },
                )?;
                Ok("Deuda guardada.".into())
            }
            Formulario::Tarjeta { id } => {
                let dia = |t: &str| -> Result<Option<u8>, String> {
                    match opcional(t) {
                        None => Ok(None),
                        Some(x) => x
                            .parse()
                            .map(Some)
                            .map_err(|_| "El día debe ser un número entre 1 y 31.".into()),
                    }
                };
                vistas::guardar_tarjeta(
                    &e.c,
                    TarjetaEntrada {
                        id: *id,
                        nombre: f.get_t1().to_string(),
                        cupo: opcional(&f.get_t2()),
                        saldo: opcional(&f.get_t3()),
                        dia_corte: dia(&f.get_t4())?,
                        dia_pago: dia(&f.get_t5())?,
                    },
                )?;
                Ok("Tarjeta guardada.".into())
            }
            Formulario::Mes { mes, .. } => {
                let mut m = mes.clone();
                m.cerrado = f.get_b1();
                vistas::guardar_mes(&e.c, m)?;
                Ok("Mes guardado.".into())
            }
            Formulario::Item {
                tipo, id, orden, ..
            } => {
                if *tipo == 0 {
                    vistas::guardar_estado(
                        &e.c,
                        EstadoGasto {
                            id: Some(*id),
                            nombre: f.get_t1().to_string(),
                            cuenta_pagado: f.get_b1(),
                            orden: *orden,
                        },
                    )?;
                } else {
                    vistas::guardar_item(
                        &e.c,
                        pantallas::catalogo_de(*tipo),
                        Item {
                            id: Some(*id),
                            nombre: f.get_t1().to_string(),
                        },
                    )?;
                }
                Ok(format!("«{}» guardado.", f.get_t1()))
            }
        }
    })();
    match r {
        Ok(msg) => {
            cerrar_form(ui, est);
            aviso(ui, est, &msg, false, None);
            recargar(ui, est);
        }
        Err(m) => f.set_error(s(m)),
    }
}

/// Pide confirmación y prepara el borrado del registro del formulario.
pub fn borrar(ui: &Ventana, est: &Est) {
    let f = ui.global::<Form>();
    let (titulo, texto, accion): (String, String, AccionConfirmada) = {
        let e = est.borrow();
        match e.form.as_ref() {
            Some(Formulario::Gasto { id: Some(id), .. }) => {
                let id = *id;
                (
                    "¿Borrar este gasto?".into(),
                    format!("{} · {}", f.get_t1(), f.get_t2()),
                    Box::new(move |e: &mut Estado| {
                        let g = vistas::borrar_gasto(&e.c, id)?;
                        Ok((
                            "Gasto borrado.".into(),
                            Box::new(move |c: &_| vistas::restaurar_gasto(c, g).map(|_| ()))
                                as Deshacer,
                        ))
                    }),
                )
            }
            Some(Formulario::Ingreso { id: Some(id), .. }) => {
                let id = *id;
                (
                    "¿Borrar este ingreso?".into(),
                    format!("Monto: {}", f.get_t2()),
                    Box::new(move |e: &mut Estado| {
                        let i = vistas::borrar_ingreso(&e.c, id)?;
                        Ok((
                            "Ingreso borrado.".into(),
                            Box::new(move |c: &_| vistas::restaurar_ingreso(c, i).map(|_| ()))
                                as Deshacer,
                        ))
                    }),
                )
            }
            Some(Formulario::Deuda { id: Some(id) }) => {
                let id = *id;
                (
                    "¿Borrar esta deuda?".into(),
                    format!(
                        "{}. Sus cuotas se conservan como gastos sin deuda.",
                        f.get_t1()
                    ),
                    Box::new(move |e: &mut Estado| {
                        let b = vistas::borrar_deuda(&e.c, id)?;
                        Ok((
                            "Deuda borrada.".into(),
                            Box::new(move |c: &_| vistas::restaurar_deuda(c, b).map(|_| ()))
                                as Deshacer,
                        ))
                    }),
                )
            }
            Some(Formulario::Tarjeta { id: Some(id) }) => {
                let id = *id;
                (
                    "¿Borrar esta tarjeta?".into(),
                    format!("{}. Sus pagos se conservan como gastos.", f.get_t1()),
                    Box::new(move |e: &mut Estado| {
                        let b = vistas::borrar_tarjeta(&e.c, id)?;
                        Ok((
                            "Tarjeta borrada.".into(),
                            Box::new(move |c: &_| vistas::restaurar_tarjeta(c, b).map(|_| ()))
                                as Deshacer,
                        ))
                    }),
                )
            }
            Some(Formulario::Mes { mes, nombre }) => {
                let (id, nombre) = (mes.id.unwrap_or(0), nombre.clone());
                (
                    format!("¿Borrar {nombre}?"),
                    "Se borran también todos sus gastos e ingresos.".into(),
                    Box::new(move |e: &mut Estado| {
                        let m = vistas::borrar_mes(&e.c, id)?;
                        Ok((
                            format!("{nombre} borrado."),
                            Box::new(move |c: &_| vistas::restaurar_mes(c, m)) as Deshacer,
                        ))
                    }),
                )
            }
            Some(Formulario::Item {
                tipo,
                id,
                reasignar,
                ..
            }) => {
                let (tipo, id) = (*tipo, *id);
                let destino = reasignar.get(f.get_s1() as usize).copied().flatten();
                (
                    format!("¿Borrar «{}»?", f.get_t1()),
                    "Esta acción no se puede deshacer.".into(),
                    Box::new(move |e: &mut Estado| {
                        if tipo == 0 {
                            vistas::borrar_estado(&e.c, id, destino)?;
                        } else {
                            vistas::borrar_item(&e.c, pantallas::catalogo_de(tipo), id, destino)?;
                        }
                        Ok((
                            "Elemento borrado.".into(),
                            Box::new(|_: &_| Ok(())) as Deshacer,
                        ))
                    }),
                )
            }
            _ => return,
        }
    };
    let deshacible = !matches!(est.borrow().form, Some(Formulario::Item { .. }));
    est.borrow_mut().confirmar = Some(Box::new(move |e: &mut Estado| {
        accion(e).map(|(m, d)| {
            (
                m,
                if deshacible {
                    d
                } else {
                    Box::new(|_: &_| Ok(())) as Deshacer
                },
            )
        })
    }));
    let c = ui.global::<Confirmar>();
    c.set_titulo(s(titulo));
    c.set_texto(s(texto));
    c.set_boton(s("Borrar"));
    c.set_visible(true);
}

pub fn confirmar_si(ui: &Ventana, est: &Est) {
    ui.global::<Confirmar>().set_visible(false);
    let accion = est.borrow_mut().confirmar.take();
    let Some(accion) = accion else { return };
    let deshacible = !matches!(est.borrow().form, Some(Formulario::Item { .. }));
    let r = accion(&mut est.borrow_mut());
    cerrar_form(ui, est);
    match r {
        Ok((msg, d)) => {
            aviso(
                ui,
                est,
                &msg,
                false,
                if deshacible { Some(d) } else { None },
            );
            recargar(ui, est);
        }
        Err(m) => aviso(ui, est, &m, true, None),
    }
}

pub fn deshacer(ui: &Ventana, est: &Est, id: i32) {
    let d = est.borrow_mut().deshacer.remove(&id);
    quitar_aviso(ui, est, id);
    if let Some(d) = d {
        let r = d(&est.borrow().c);
        match r {
            Ok(()) => aviso(ui, est, "Cambio deshecho.", false, None),
            Err(m) => aviso(ui, est, &m, true, None),
        }
        recargar(ui, est);
    }
}

/// Marca un gasto como pagado (primer estado que cuenta como pagado), con deshacer.
pub fn marcar_pagado(ui: &Ventana, est: &Est, id: i64) {
    let r = (|| -> Result<(String, Deshacer), String> {
        let e = est.borrow();
        let pagado = repo::listar_estados(&e.c)
            .map_err(|x| x.to_string())?
            .into_iter()
            .find(|x| x.cuenta_pagado)
            .ok_or("No hay ningún estado que cuente como pagado. Revísalo en Catálogos.")?;
        let g = repo::gasto(&e.c, id).map_err(|x| x.to_string())?;
        let anterior = g.clone();
        let mut nuevo = g;
        nuevo.estado_id = pagado.id.unwrap_or(0);
        repo::guardar_gasto(&e.c, &nuevo).map_err(|x| x.to_string())?;
        Ok((
            format!("«{}» marcado como {}.", anterior.concepto, pagado.nombre),
            Box::new(move |c: &_| {
                repo::guardar_gasto(c, &anterior)
                    .map(|_| ())
                    .map_err(|x| x.to_string())
            }),
        ))
    })();
    match r {
        Ok((m, d)) => {
            aviso(ui, est, &m, false, Some(d));
            recargar(ui, est);
        }
        Err(m) => aviso(ui, est, &m, true, None),
    }
}

pub fn agregar_item(ui: &Ventana, est: &Est, tipo: i32, nombre: &str) {
    let r = {
        let e = est.borrow();
        if tipo == 0 {
            vistas::guardar_estado(
                &e.c,
                EstadoGasto {
                    id: None,
                    nombre: nombre.into(),
                    cuenta_pagado: false,
                    orden: 50,
                },
            )
            .map(|_| ())
        } else {
            vistas::guardar_item(
                &e.c,
                pantallas::catalogo_de(tipo),
                Item {
                    id: None,
                    nombre: nombre.into(),
                },
            )
            .map(|_| ())
        }
    };
    match r {
        Ok(()) => {
            aviso(
                ui,
                est,
                &format!("«{}» añadido.", nombre.trim()),
                false,
                None,
            );
            recargar(ui, est);
        }
        Err(m) => aviso(ui, est, &m, true, None),
    }
}

// ---------- Importar y exportar ----------

fn filtro_ods(d: rfd::FileDialog) -> rfd::FileDialog {
    d.add_filter("Hoja de cálculo OpenDocument", &["ods"])
}

pub fn importar(ui: &Ventana, est: &Est, forzar: bool) {
    let Some(ruta) =
        filtro_ods(rfd::FileDialog::new().set_title("Elige la hoja de cálculo")).pick_file()
    else {
        return;
    };
    let r = sueldapp_ods::importar::importar_archivo(&mut est.borrow_mut().c, &ruta, forzar);
    match r {
        Ok(inf) => {
            let ok = inf.ok;
            est.borrow_mut().ultimo_informe = Some(inf);
            {
                let mut e = est.borrow_mut();
                e.cursores.clear();
                e.inicios.clear();
                e.rango = None;
            }
            aviso(
                ui,
                est,
                if ok {
                    "Importación terminada."
                } else {
                    "La importación falló; revisa el informe."
                },
                !ok,
                None,
            );
            recargar(ui, est);
        }
        Err(e) => aviso(ui, est, &e.to_string(), true, None),
    }
}

fn guardar_como(titulo: &str, nombre: &str) -> Option<std::path::PathBuf> {
    let mut r = filtro_ods(
        rfd::FileDialog::new()
            .set_title(titulo)
            .set_file_name(nombre),
    )
    .save_file()?;
    if r.extension().is_none() {
        r.set_extension("ods");
    }
    Some(r)
}

pub fn plantilla(ui: &Ventana, est: &Est) {
    let Some(ruta) = guardar_como("Guardar plantilla para llenar", "sueldapp-plantilla.ods") else {
        return;
    };
    match sueldapp_ods::escritura::exportar_plantilla(&ruta) {
        Ok(()) => aviso(
            ui,
            est,
            &format!("Plantilla guardada en {}", ruta.display()),
            false,
            None,
        ),
        Err(e) => aviso(ui, est, &e.to_string(), true, None),
    }
}

pub fn exportar_ods(ui: &Ventana, est: &Est) {
    let Some(ruta) = guardar_como("Guardar exportación", "sueldapp-exportacion.ods") else {
        return;
    };
    let r = sueldapp_ods::escritura::exportar_ods(&est.borrow().c, &ruta);
    match r {
        Ok(()) => aviso(
            ui,
            est,
            &format!("Exportado en {}", ruta.display()),
            false,
            None,
        ),
        Err(e) => aviso(ui, est, &e.to_string(), true, None),
    }
}

pub fn exportar_csv(ui: &Ventana, est: &Est) {
    let Some(carpeta) = rfd::FileDialog::new()
        .set_title("Elige la carpeta para los CSV")
        .pick_folder()
    else {
        return;
    };
    let r = sueldapp_ods::csv::exportar_csv(&est.borrow().c, &carpeta);
    match r {
        Ok(v) => aviso(
            ui,
            est,
            &format!("{} archivos CSV guardados.", v.len()),
            false,
            None,
        ),
        Err(e) => aviso(ui, est, &e.to_string(), true, None),
    }
}

/// Conecta un callback de la interfaz con una acción que recibe la ventana y el estado.
pub fn con<F: Fn(&Ventana, &Est) + 'static>(ui: &Ventana, est: &Est, f: F) -> impl Fn() + 'static {
    let (debil, est): (Weak<Ventana>, Est) = (ui.as_weak(), est.clone());
    move || {
        if let Some(ui) = debil.upgrade() {
            f(&ui, &est);
        }
    }
}
