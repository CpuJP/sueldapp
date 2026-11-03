//! Capturas de todas las pantallas en los 4 modos (claro/oscuro × vidrio/sólido), para revisar
//! el diseño y la documentación: `SUELDAPP_CAPTURAS=<carpeta>`. Usa `Window::take_snapshot`,
//! así que no depende de herramientas del sistema. Al terminar cierra la app.

use crate::acciones::{abrir_gasto, navegar, Est};
use crate::{App, Form, Tema, Ventana};
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
    /// Tooltip de una barra con el puntero encima (sin pulsar).
    Tooltip,
    /// Las tres clases de notificación a la vez.
    Notificaciones,
    DeudaAbierta,
    ResumenAnio,
    Dialogo,
    Alertas,
    TableroEditable,
    TableroEditor,
    /// Pagar lo último de un mes: pregunta, botón de Resumen y reapertura.
    /// Pantalla con la ventana de un tamaño dado.
    Tamano(f32, f32, i32, String),
    /// Formularios en la ventana más pequeña.
    DialogoMovil,
    AlertasMovil,
    PruebaMenu,
    Arrastre,
    Soltar,
    MesPregunta,
    MesPorConfirmar,
    MesHecho,
    MesReabierto,
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
        (8, "meses"),
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
            Paso::Tooltip,
            Paso::Notificaciones,
            Paso::DeudaAbierta,
            Paso::ResumenAnio,
            Paso::Dialogo,
            Paso::Alertas,
            Paso::TableroEditable,
            Paso::TableroEditor,
        ]);
    }
    pasos.push(Paso::Prueba);
    pasos.extend([
        Paso::Arrastre,
        Paso::Soltar,
        Paso::MesPregunta,
        Paso::MesPorConfirmar,
        Paso::MesHecho,
        Paso::MesReabierto,
    ]);
    // Al final: cambian el tamaño de la ventana y las pruebas de ratón usan posiciones fijas.
    pasos.push(Paso::PruebaMenu);
    for (ancho, alto, nombre) in [
        (360.0, 640.0, "movil"),
        (600.0, 800.0, "estrecha"),
        (768.0, 1024.0, "tableta"),
        (1024.0, 700.0, "media"),
        (800.0, 420.0, "baja"),
        (1920.0, 1080.0, "ancha"),
    ] {
        for (v, n) in vistas {
            pasos.push(Paso::Tamano(
                ancho,
                alto,
                v,
                format!("adaptar-{nombre}-{n}"),
            ));
        }
    }
    pasos.extend([Paso::DialogoMovil, Paso::AlertasMovil]);
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
                ui.global::<App>().set_alertas_abiertas(false);
                let t = ui.global::<crate::TableroD>();
                if t.get_arr_tipo() < 0 {
                    t.set_editando(false);
                    t.set_editor_abierto(false);
                }
                ui.window().set_size(slint::LogicalSize::new(1280., 840.));
                est.borrow_mut().abiertas.clear();
                let ids: Vec<i32> = est.borrow().avisos.iter().map(|a| a.id).collect();
                for id in ids {
                    crate::notificaciones::quitar(&ui, &est, id);
                }
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
                Some(Paso::Tooltip) => {
                    ui.window().set_size(slint::LogicalSize::new(1280., 1900.));
                    navegar(&ui, &est, 0);
                    ui.window()
                        .dispatch_event(slint::platform::WindowEvent::PointerMoved {
                            position: slint::LogicalPosition::new(560., 880.),
                        });
                    *pendiente.borrow_mut() = Some(format!("tooltip-{}", sufijo.borrow()));
                }
                Some(Paso::Notificaciones) => {
                    use crate::notificaciones::{notificar, Tipo};
                    ui.window()
                        .dispatch_event(slint::platform::WindowEvent::PointerExited);
                    navegar(&ui, &est, 1);
                    notificar(
                        &ui,
                        &est,
                        Tipo::Error,
                        "No se pudo guardar: el monto no es válido.",
                        None,
                    );
                    notificar(&ui, &est, Tipo::Advertencia, "Cambio deshecho.", None);
                    notificar(
                        &ui,
                        &est,
                        Tipo::Exito,
                        "«Portátil – Cuota 4» marcado como Pagado.",
                        Some(Box::new(|_| Ok(()))),
                    );
                    *pendiente.borrow_mut() = Some(format!("notificaciones-{}", sufijo.borrow()));
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
                Some(Paso::Alertas) => {
                    navegar(&ui, &est, 1);
                    ui.global::<App>().set_alertas_abiertas(true);
                    *pendiente.borrow_mut() = Some(format!("alertas-{}", sufijo.borrow()));
                }
                Some(Paso::TableroEditable) => {
                    ui.global::<crate::TableroD>().set_editando(true);
                    navegar(&ui, &est, 0);
                    *pendiente.borrow_mut() = Some(format!("tablero-editar-{}", sufijo.borrow()));
                }
                Some(Paso::TableroEditor) => {
                    let t = ui.global::<crate::TableroD>();
                    t.set_editando(true);
                    navegar(&ui, &est, 0);
                    t.invoke_abrir_editor(6);
                    *pendiente.borrow_mut() = Some(format!("tablero-modulo-{}", sufijo.borrow()));
                }
                Some(Paso::Tamano(ancho, alto, v, nombre)) => {
                    tamano(&ui, ancho, alto);
                    navegar(&ui, &est, v);
                    *pendiente.borrow_mut() = Some(nombre);
                }
                Some(Paso::DialogoMovil) => {
                    tamano(&ui, 360., 640.);
                    navegar(&ui, &est, 1);
                    let id = est.borrow().visibles.keys().min().copied();
                    abrir_gasto(&ui, &est, id, None, None);
                    *pendiente.borrow_mut() = Some("adaptar-movil-dialogo".to_string());
                }
                Some(Paso::AlertasMovil) => {
                    tamano(&ui, 360., 640.);
                    navegar(&ui, &est, 1);
                    ui.global::<App>().set_alertas_abiertas(true);
                    *pendiente.borrow_mut() = Some("adaptar-movil-alertas".to_string());
                }
                Some(Paso::PruebaMenu) => {
                    if let Err(e) = probar_menu(&ui) {
                        eprintln!("menú: {e}");
                        let _ = std::fs::write(carpeta.join("prueba-menu-fallo.txt"), &e);
                    }
                }
                Some(Paso::Arrastre) => match iniciar_arrastre(&ui, &est) {
                    Ok(()) => *pendiente.borrow_mut() = Some("tablero-arrastre".to_string()),
                    Err(e) => {
                        eprintln!("arrastre: {e}");
                        let _ = std::fs::write(carpeta.join("prueba-arrastre-fallo.txt"), &e);
                    }
                },
                Some(Paso::Soltar) => {
                    if let Err(e) = probar_gestos(&ui, &est) {
                        eprintln!("gestos: {e}");
                        let _ = std::fs::write(carpeta.join("prueba-gestos-fallo.txt"), &e);
                    }
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
                Some(
                    p @ (Paso::MesPregunta
                    | Paso::MesPorConfirmar
                    | Paso::MesHecho
                    | Paso::MesReabierto),
                ) => {
                    let (r, nombre) = match p {
                        Paso::MesPregunta => (mes_pregunta(&ui, &est), "mes-pregunta"),
                        Paso::MesPorConfirmar => {
                            (mes_por_confirmar(&ui, &est), "mes-por-confirmar")
                        }
                        Paso::MesHecho => (mes_hecho(&ui, &est), "mes-hecho"),
                        _ => (mes_reabierto(&ui, &est), "mes-reabierto"),
                    };
                    match r {
                        Ok(()) => *pendiente.borrow_mut() = Some(nombre.to_string()),
                        Err(e) => {
                            eprintln!("prueba de mes: {e}");
                            let _ = std::fs::write(carpeta.join("prueba-mes-fallo.txt"), &e);
                        }
                    }
                }
                Some(Paso::Cerrar) | None => {
                    t2.stop();
                    let _ = slint::quit_event_loop();
                }
            }
        },
    );
}

fn tamano(ui: &Ventana, w: f32, h: f32) {
    let size = slint::LogicalSize::new(w, h);
    ui.window().set_size(size);
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::Resized { size });
    // `changed width` corre en el siguiente ciclo de eventos; la prueba lo necesita ya.
    ui.global::<crate::Disp>().set_ventana(w);
    ui.global::<crate::Disp>().set_alto(h);
}

/// Colapsa y expande el menú con clics reales, y comprueba el modo automático.
fn probar_menu(ui: &Ventana) -> Result<(), String> {
    use crate::Disp;
    let d = ui.global::<Disp>();
    let clic = |x: f32, y: f32| {
        pulsar(ui, x, y);
        soltar(ui, x, y);
    };
    tamano(ui, 1280.0, 840.0);
    if d.get_compacto() {
        return Err("el menú debe empezar completo".into());
    }
    for x in (216..276).step_by(6) {
        clic(x as f32, 50.0);
        if d.get_compacto() {
            break;
        }
    }
    if !d.get_compacto() || d.get_ancho_menu() != 72.0 {
        return Err("el botón no contrajo el menú".into());
    }
    if !crate::estado::leer_ajustes().menu_compacto {
        return Err("no se guardó el menú compacto".into());
    }
    for y in (60..140).step_by(6) {
        clic(54.0, y as f32);
        if !d.get_compacto() {
            break;
        }
    }
    if d.get_compacto() {
        return Err("el botón no expandió el menú".into());
    }
    // Ventana estrecha: se compacta sola y el botón lo abre encima, sin cambiar la preferencia.
    tamano(ui, 700.0, 700.0);
    if !d.get_forzado() || !d.get_compacto() || d.get_menu_compacto() {
        return Err("no se compactó solo".into());
    }
    for y in (60..140).step_by(6) {
        clic(54.0, y as f32);
        if d.get_menu_abierto() {
            break;
        }
    }
    if !d.get_menu_abierto() {
        return Err("el botón no abrió el menú encima".into());
    }
    tamano(ui, 1280.0, 840.0);
    // El menú encima solo se dibuja con la ventana estrecha; `changed width` limpia la marca después.
    if d.get_forzado() || d.get_compacto() {
        return Err("al ensanchar debe volver a la forma normal".into());
    }
    Ok(())
}

fn raton(ui: &Ventana, ev: slint::platform::WindowEvent) {
    ui.window().dispatch_event(ev);
}

fn pulsar(ui: &Ventana, x: f32, y: f32) {
    use slint::platform::{PointerEventButton, WindowEvent};
    let position = slint::LogicalPosition::new(x, y);
    raton(ui, WindowEvent::PointerMoved { position });
    raton(
        ui,
        WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        },
    );
}

fn mover(ui: &Ventana, x: f32, y: f32) {
    raton(
        ui,
        slint::platform::WindowEvent::PointerMoved {
            position: slint::LogicalPosition::new(x, y),
        },
    );
}

fn soltar(ui: &Ventana, x: f32, y: f32) {
    use slint::platform::{PointerEventButton, WindowEvent};
    raton(
        ui,
        WindowEvent::PointerReleased {
            position: slint::LogicalPosition::new(x, y),
            button: PointerEventButton::Left,
        },
    );
}

/// Arrastra con eventos de ratón reales el primer módulo sobre el siguiente, sin soltarlo.
fn iniciar_arrastre(ui: &Ventana, est: &Est) -> Result<(), String> {
    let t = ui.global::<crate::TableroD>();
    t.set_editando(true);
    navegar(ui, est, 0);
    pulsar(ui, 500.0, 300.0);
    mover(ui, 520.0, 400.0);
    mover(ui, 620.0, 620.0);
    if t.get_arr_tipo() != 0 {
        return Err(format!("no empezó a arrastrar: {}", t.get_arr_tipo()));
    }
    Ok(())
}

/// Termina el arrastre y prueba con ratón los bordes y el botón de quitar.
fn probar_gestos(ui: &Ventana, est: &Est) -> Result<(), String> {
    let t = ui.global::<crate::TableroD>();
    let cfg = |est: &Est| est.borrow().tablero.clone();
    soltar(ui, 620.0, 620.0);
    if t.get_arr_tipo() != -1 {
        return Err("el arrastre no terminó".into());
    }
    let primero = cfg(est).modulos[0].tipo;
    if primero == 0 {
        return Err("soltar no cambió el orden".into());
    }

    // Borde derecho del primer módulo: buscar su altura, porque depende del contenido.
    let derecha = 1233.0;
    let ancho = |est: &Est| cfg(est).modulos[0].ancho;
    let inicial = ancho(est);
    for y in (240..460).step_by(12) {
        pulsar(ui, derecha, y as f32);
        mover(ui, 720.0, y as f32);
        soltar(ui, 720.0, y as f32);
        if ancho(est) != inicial {
            break;
        }
    }
    if ancho(est) == inicial {
        return Err("el borde derecho no cambió el ancho".into());
    }

    // Borde inferior de un gráfico.
    t.invoke_restablecer();
    ui.global::<crate::Confirmar>().invoke_si();
    // Solo el gráfico: pulsar fuera del borde arrastraría el módulo sobre otro.
    for k in (0..14).filter(|k| *k != 3) {
        t.invoke_ocultar(k, false);
    }
    let alto = |est: &Est| cfg(est).modulo(3).map(|m| m.tam);
    let tam = alto(est);
    for y in (360..760).step_by(8) {
        pulsar(ui, 760.0, y as f32);
        mover(ui, 760.0, (y + 80) as f32);
        soltar(ui, 760.0, (y + 80) as f32);
        if alto(est) != tam {
            break;
        }
    }
    if alto(est) == tam {
        return Err("el borde inferior no cambió el tamaño".into());
    }

    // Botón − del primer módulo visible.
    let visibles = |est: &Est| cfg(est).modulos.iter().filter(|m| m.visible).count();
    let n = visibles(est);
    for y in (190..320).step_by(4) {
        pulsar(ui, 302.0, y as f32);
        soltar(ui, 302.0, y as f32);
        if visibles(est) != n {
            break;
        }
    }
    if visibles(est) != n - 1 {
        return Err("el botón − no quitó el módulo".into());
    }

    t.invoke_restablecer();
    ui.global::<crate::Confirmar>().invoke_si();
    t.set_editando(false);
    navegar(ui, est, 0);
    Ok(())
}

/// Alertas y tablero configurables: mismos callbacks que los controles de la interfaz.
fn prueba_configuracion(ui: &Ventana, est: &Est) -> Result<(), String> {
    use crate::{Confirmar, TableroD};
    use slint::Model;
    let app = ui.global::<App>();
    navegar(ui, est, 0);
    if app.get_alertas().row_count() == 0 {
        return Err("el modo demo debe tener alertas".into());
    }
    app.set_alerta_vencidos(false);
    app.invoke_ajustes_cambiados();
    if app.get_alertas().row_count() != 0 {
        return Err("sin vencidos no debe quedar la alerta vencida".into());
    }
    app.set_alerta_vencidos(true);
    app.set_alerta_activas(false);
    app.invoke_ajustes_cambiados();
    if app.get_alertas().row_count() != 0 {
        return Err("desactivar no quitó las alertas".into());
    }
    app.set_alerta_activas(true);
    app.invoke_ajustes_cambiados();
    if crate::estado::leer_ajustes().alertas_activas != app.get_alerta_activas() {
        return Err("los ajustes de alertas no se guardaron".into());
    }

    let t = ui.global::<TableroD>();
    t.invoke_ocultar(1, false);
    if t.get_ocultos().row_count() != 1 {
        return Err("ocultar no funcionó".into());
    }
    t.invoke_ocultar(1, true);
    t.invoke_cambiar_ancho(3, 1);
    t.invoke_mover(4, true);
    let cuatro_primero = t
        .get_filas()
        .iter()
        .flat_map(|f| f.modulos.iter().collect::<Vec<_>>())
        .map(|m| m.tipo)
        .collect::<Vec<_>>();
    if cuatro_primero.get(3) != Some(&4) {
        return Err(format!("mover no funcionó: {cuatro_primero:?}"));
    }
    t.invoke_cambiar_tam(6, 0);
    if t.get_destino().row_count() > 3 {
        return Err("el tamaño no limitó la lista".into());
    }
    if crate::tablero_config::leer() != est.borrow().tablero {
        return Err("la configuración no se guardó".into());
    }
    t.invoke_restablecer();
    if !ui.global::<Confirmar>().get_visible() {
        return Err("restablecer no pidió confirmación".into());
    }
    ui.global::<Confirmar>().invoke_si();
    if est.borrow().tablero != Default::default() {
        return Err("restablecer no funcionó".into());
    }
    Ok(())
}

const MES_PRUEBA: (i32, u32) = (2026, 12);

fn mes_prueba(est: &Est) -> Result<sueldapp_dominio::Mes, String> {
    sueldapp_almacen::repo::mes_por_periodo(&est.borrow().c, MES_PRUEBA.0, MES_PRUEBA.1)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "sin mes de prueba".to_string())
}

fn mes_pregunta(ui: &Ventana, est: &Est) -> Result<(), String> {
    use crate::{Confirmar, ResumenD};
    est.borrow_mut().resumen = MES_PRUEBA;
    navegar(ui, est, 1);
    let id = mes_prueba(est)?.id.ok_or("mes sin id")?;
    let pendientes = {
        let e = est.borrow();
        let estados = sueldapp_almacen::repo::listar_estados(&e.c).map_err(|x| x.to_string())?;
        sueldapp_almacen::repo::gastos_de_mes(&e.c, id)
            .map_err(|x| x.to_string())?
            .into_iter()
            .filter(|g| {
                !estados
                    .iter()
                    .any(|x| x.id == Some(g.estado_id) && x.cuenta_pagado)
            })
            .filter_map(|g| g.id)
            .collect::<Vec<_>>()
    };
    let (ultimo, resto) = pendientes.split_last().ok_or("sin pendientes")?;
    for g in resto {
        ui.global::<ResumenD>().invoke_marcar_pagado(*g as i32);
    }
    if ui.global::<Confirmar>().get_visible() {
        return Err("preguntó con pagos pendientes".into());
    }
    if mes_prueba(est)?.cerrado {
        return Err("se marcó sin confirmar".into());
    }
    ui.global::<ResumenD>().invoke_marcar_pagado(*ultimo as i32);
    if !ui.global::<Confirmar>().get_visible() {
        return Err("no preguntó al pagar lo último".into());
    }
    Ok(())
}

fn mes_por_confirmar(ui: &Ventana, est: &Est) -> Result<(), String> {
    use crate::{Confirmar, ResumenD};
    ui.global::<Confirmar>().invoke_no();
    est.borrow_mut().resumen = MES_PRUEBA;
    navegar(ui, est, 1);
    if mes_prueba(est)?.cerrado || !ui.global::<ResumenD>().get_por_confirmar() {
        return Err("«ahora no» debe dejarlo por confirmar".into());
    }
    if ui.global::<Confirmar>().get_visible() {
        return Err("volvió a preguntar".into());
    }
    Ok(())
}

fn mes_hecho(ui: &Ventana, est: &Est) -> Result<(), String> {
    use crate::{MesesD, ResumenD};
    // «Ver mes» del módulo Meses abre el mes en Resumen.
    navegar(ui, est, 8);
    est.borrow_mut().resumen = (2026, 7);
    ui.global::<MesesD>()
        .invoke_abrir(crate::estado::clave(MES_PRUEBA));
    if ui.global::<App>().get_vista() != 1 || est.borrow().resumen != MES_PRUEBA {
        return Err("«Ver mes» no abrió el mes".into());
    }
    if slint::Model::row_count(&ui.global::<MesesD>().get_por_confirmar()) != 1 {
        navegar(ui, est, 8);
    }
    if slint::Model::row_count(&ui.global::<MesesD>().get_por_confirmar()) != 1 {
        return Err("Meses no lista el mes por confirmar".into());
    }
    navegar(ui, est, 1);
    ui.global::<ResumenD>().invoke_marcar_hecho();
    if !mes_prueba(est)?.cerrado || ui.global::<ResumenD>().get_situacion() != "Hecho" {
        return Err("el botón no lo marcó".into());
    }
    Ok(())
}

fn mes_reabierto(ui: &Ventana, est: &Est) -> Result<(), String> {
    use crate::ResumenD;
    est.borrow_mut().resumen = MES_PRUEBA;
    navegar(ui, est, 1);
    // Un gasto nuevo nace pendiente.
    ui.global::<App>().invoke_nuevo_gasto();
    let f = ui.global::<Form>();
    f.set_t1("Pago nuevo".into());
    f.set_t2("1000".into());
    f.invoke_guardar();
    if f.get_tipo() != 0 {
        return Err(format!("no guardó: {}", f.get_error()));
    }
    if mes_prueba(est)?.cerrado || ui.global::<ResumenD>().get_situacion() != "Falta 1 pago" {
        return Err("un pago nuevo debe reabrir el mes".into());
    }
    // Y con pagos pendientes el formulario de Editar mes lo rechaza.
    crate::acciones::abrir_mes(ui, est);
    f.set_b1(true);
    f.invoke_guardar();
    if f.get_tipo() == 0 || f.get_error().is_empty() || mes_prueba(est)?.cerrado {
        return Err("Editar mes aceptó un mes con pendientes".into());
    }
    Ok(())
}

/// Acciona los mismos callbacks que los botones y comprueba el efecto en la base.
fn prueba_de_humo(ui: &Ventana, est: &Est) -> Result<(), String> {
    use crate::{Confirmar, ResumenD};
    let contar = |est: &Est| -> Result<usize, String> {
        sueldapp_almacen::repo::todos_gastos(&est.borrow().c)
            .map(|v| v.len())
            .map_err(|e| e.to_string())
    };
    prueba_configuracion(ui, est)?;
    let antes = contar(est)?;
    // Pagar una alerta la quita de la lista, y deshacer la devuelve.
    let alertas = ui.global::<App>().get_alertas();
    let n = slint::Model::row_count(&alertas);
    let primera = slint::Model::row_data(&alertas, 0).ok_or("el modo demo debe tener alertas")?;
    ui.global::<ResumenD>().invoke_marcar_pagado(primera.id);
    if slint::Model::row_count(&ui.global::<App>().get_alertas()) != n - 1 {
        return Err("pagar no quitó la alerta".into());
    }
    let aviso = est
        .borrow()
        .avisos
        .last()
        .map(|a| a.id)
        .ok_or("sin aviso")?;
    ui.global::<App>().invoke_deshacer(aviso);
    if slint::Model::row_count(&ui.global::<App>().get_alertas()) != n {
        return Err("deshacer no devolvió la alerta".into());
    }
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
