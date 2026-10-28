//! Capturas de todas las pantallas en los 4 modos (claro/oscuro × vidrio/sólido), para revisar
//! el diseño y la documentación: `SUELDAPP_CAPTURAS=<carpeta>`. Usa `Window::take_snapshot`,
//! así que no depende de herramientas del sistema. Al terminar cierra la app.

use crate::acciones::{abrir_gasto, navegar, Est};
use crate::{Form, Tema, Ventana};
use slint::{ComponentHandle, Timer};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

enum Paso {
    Prueba,
    Modo(i32, bool, &'static str),
    Vista(i32, &'static str),
    /// Tablero completo, con la ventana alta para que entren todos los gráficos.
    TableroAlto,
    DeudaAbierta,
    ResumenAnio,
    Dialogo,
    Cerrar,
}

fn guardar(ui: &Ventana, ruta: &PathBuf) -> Result<(), String> {
    let img = ui.window().take_snapshot().map_err(|e| e.to_string())?;
    let f = std::fs::File::create(ruta).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), img.width(), img.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut w| w.write_image_data(img.as_bytes()))
        .map_err(|e| e.to_string())
}

pub fn iniciar(ui: &Ventana, est: &Est, carpeta: PathBuf) {
    let _ = std::fs::create_dir_all(&carpeta);
    let vistas = [
        (0, "tablero"),
        (1, "resumen"),
        (2, "movimientos"),
        (3, "deudas"),
        (4, "tarjetas"),
        (5, "catalogos"),
        (6, "datos"),
        (7, "ajustes"),
    ];
    let mut pasos = Vec::new();
    for (modo, solido, nombre) in [
        (1, false, "claro"),
        (2, false, "oscuro"),
        (1, true, "claro-solido"),
        (2, true, "oscuro-solido"),
    ] {
        pasos.push(Paso::Modo(modo, solido, nombre));
        for (v, n) in vistas {
            pasos.push(Paso::Vista(v, n));
        }
        pasos.extend([
            Paso::TableroAlto,
            Paso::DeudaAbierta,
            Paso::ResumenAnio,
            Paso::Dialogo,
        ]);
    }
    pasos.push(Paso::Prueba);
    pasos.push(Paso::Cerrar);
    pasos.reverse();

    let pasos = Rc::new(RefCell::new(pasos));
    let sufijo = Rc::new(RefCell::new(String::new()));
    let (debil, est) = (ui.as_weak(), est.clone());
    let temporizador = Rc::new(Timer::default());
    let t2 = temporizador.clone();
    // Cada 700 ms: captura lo que se ve y avanza al siguiente paso.
    let pendiente: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    temporizador.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(700),
        move || {
            let Some(ui) = debil.upgrade() else { return };
            if let Some(nombre) = pendiente.borrow_mut().take() {
                if let Err(e) = guardar(&ui, &carpeta.join(format!("{nombre}.png"))) {
                    eprintln!("captura {nombre}: {e}");
                }
                ui.global::<Form>().set_tipo(0);
                ui.window().set_size(slint::LogicalSize::new(1280., 840.));
                est.borrow_mut().abiertas.clear();
                est.borrow_mut().modo_resumen = 0;
            }
            let paso = pasos.borrow_mut().pop();
            match paso {
                Some(Paso::Modo(modo, solido, n)) => {
                    let t = ui.global::<Tema>();
                    t.set_modo(modo);
                    t.set_solido(solido);
                    t.set_sin_movimiento(true);
                    *sufijo.borrow_mut() = n.to_string();
                }
                Some(Paso::Vista(v, n)) => {
                    navegar(&ui, &est, v);
                    *pendiente.borrow_mut() = Some(format!("{n}-{}", sufijo.borrow()));
                }
                Some(Paso::TableroAlto) => {
                    ui.window().set_size(slint::LogicalSize::new(1280., 1900.));
                    navegar(&ui, &est, 0);
                    *pendiente.borrow_mut() = Some(format!("tablero-2-{}", sufijo.borrow()));
                }
                Some(Paso::DeudaAbierta) => {
                    let id = crate::vistas::listar_deudas(&est.borrow().c)
                        .ok()
                        .and_then(|v| v.first().and_then(|d| d.deuda.id));
                    est.borrow_mut().abiertas.extend(id);
                    navegar(&ui, &est, 3);
                    *pendiente.borrow_mut() = Some(format!("deudas-detalle-{}", sufijo.borrow()));
                }
                Some(Paso::ResumenAnio) => {
                    est.borrow_mut().modo_resumen = 1;
                    navegar(&ui, &est, 1);
                    *pendiente.borrow_mut() = Some(format!("resumen-anio-{}", sufijo.borrow()));
                }
                Some(Paso::Dialogo) => {
                    navegar(&ui, &est, 1);
                    let id = est.borrow().visibles.keys().min().copied();
                    abrir_gasto(&ui, &est, id, None, None);
                    *pendiente.borrow_mut() = Some(format!("dialogo-gasto-{}", sufijo.borrow()));
                }
                Some(Paso::Prueba) => {
                    let r = prueba_de_humo(&ui, &est);
                    let nombre = match &r {
                        Ok(()) => "prueba-ok".to_string(),
                        Err(e) => format!(
                            "prueba-fallo-{}",
                            e.replace(|c: char| !c.is_alphanumeric(), "-")
                        ),
                    };
                    if let Err(e) = &r {
                        eprintln!("prueba de humo: {e}");
                    }
                    let _ = std::fs::write(carpeta.join(format!("{nombre}.txt")), "");
                }
                Some(Paso::Cerrar) | None => {
                    t2.stop();
                    let _ = slint::quit_event_loop();
                }
            }
        },
    );
}

/// Acciona los mismos callbacks que los botones y comprueba el efecto en la base.
fn prueba_de_humo(ui: &Ventana, est: &Est) -> Result<(), String> {
    use crate::{App, Confirmar, ResumenD};
    let contar = |est: &Est| -> Result<usize, String> {
        sueldapp_almacen::repo::todos_gastos(&est.borrow().c)
            .map(|v| v.len())
            .map_err(|e| e.to_string())
    };
    let antes = contar(est)?;
    // Nuevo gasto desde el formulario, con la máscara de montos.
    ui.global::<App>().invoke_nuevo_gasto();
    let f = ui.global::<Form>();
    f.set_t1("Prueba de humo".into());
    f.set_t2(crate::mascara::formatear_entrada("1234567,891").into());
    f.invoke_guardar();
    if f.get_tipo() != 0 {
        return Err(format!("el formulario no se cerró: {}", f.get_error()));
    }
    if contar(est)? != antes + 1 {
        return Err("el gasto no se guardó".into());
    }
    let nuevo = sueldapp_almacen::repo::todos_gastos(&est.borrow().c)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|g| g.concepto == "Prueba de humo")
        .ok_or("no aparece el gasto")?;
    if nuevo.monto.to_string() != "1234567.89" {
        return Err(format!("monto mal leído: {}", nuevo.monto));
    }
    // Marcar pagado y deshacer.
    ui.global::<ResumenD>()
        .invoke_marcar_pagado(nuevo.id.unwrap_or(0) as i32);
    let aviso = est
        .borrow()
        .avisos
        .last()
        .map(|a| a.id)
        .ok_or("sin aviso")?;
    ui.global::<App>().invoke_deshacer(aviso);
    // Borrar con confirmación y deshacer.
    abrir_gasto(ui, est, nuevo.id, None, None);
    ui.global::<Form>().invoke_borrar();
    if !ui.global::<Confirmar>().get_visible() {
        return Err("no pidió confirmación".into());
    }
    ui.global::<Confirmar>().invoke_si();
    if contar(est)? != antes {
        return Err("no se borró".into());
    }
    let aviso = est
        .borrow()
        .avisos
        .last()
        .map(|a| a.id)
        .ok_or("sin aviso al borrar")?;
    ui.global::<App>().invoke_deshacer(aviso);
    if contar(est)? != antes + 1 {
        return Err("deshacer no restauró".into());
    }
    Ok(())
}
