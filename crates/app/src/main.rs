// Sin consola extra en Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Código generado por Slint a partir de `ui/*.slint` (su layout usa coma flotante: es
/// geometría de pantalla, nunca dinero).
#[allow(clippy::float_arithmetic)]
mod interfaz {
    slint::include_modules!();
}
pub use interfaz::*;

mod acciones;
mod capturas;
mod comun;
mod demo;
mod estado;
mod geometria;
mod mascara;
mod notificaciones;
mod p_tablero;
mod pantallas;
mod vistas;

use acciones::{con, recargar, Est};
use estado::{clave, de_clave, hoy, leer_ajustes, Estado, Rango};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;

fn main() -> Result<(), slint::PlatformError> {
    let ruta = sueldapp_almacen::ruta_por_defecto();
    let c = match sueldapp_almacen::abrir(&ruta) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("No se pudo abrir la base {}: {e}", ruta.display());
            std::process::exit(1);
        }
    };
    let demo_activo = std::env::var_os("SUELDAPP_DEMO").is_some();
    if demo_activo {
        if let Err(e) = demo::sembrar(&c) {
            eprintln!("No se pudieron crear los datos de demostración: {e}");
        }
    }
    if std::env::var_os("SUELDAPP_CAPTURAS").is_some() {
        // take_snapshot solo funciona con el renderizador por software.
        std::env::set_var("SLINT_BACKEND", "winit-software");
    }
    let ui = Ventana::new()?;
    let est: Est = Rc::new(RefCell::new(Estado::nuevo(c)));
    if demo_activo {
        est.borrow_mut().resumen = (2026, 10);
    }

    // Ajustes guardados.
    let a = leer_ajustes();
    let tema = ui.global::<Tema>();
    tema.set_modo(match std::env::var("SUELDAPP_TEMA").as_deref() {
        Ok("claro") => 1,
        Ok("oscuro") => 2,
        _ => a.tema,
    });
    tema.set_solido(a.solido);
    tema.set_sin_movimiento(a.sin_movimiento);
    est.borrow_mut().tam_indice = a.tam;
    let app = ui.global::<App>();
    app.set_demo(demo_activo);
    app.set_ruta_base(ruta.display().to_string().into());

    conectar(&ui, &est);
    acciones::navegar(&ui, &est, 0);

    if let Some(carpeta) = std::env::var_os("SUELDAPP_CAPTURAS") {
        capturas::iniciar(&ui, &est, carpeta.into());
    }
    ui.run()
}

/// Atajo para callbacks con un argumento: clona el estado y la ventana débil.
macro_rules! cb {
    ($ui:expr, $est:expr, |$u:ident, $e:ident $(, $a:ident)*| $cuerpo:expr) => {{
        let (est, debil) = ($est.clone(), $ui.as_weak());
        move |$($a),*| {
            if let Some($u) = debil.upgrade() {
                let $e = &est;
                let $u = &$u;
                $cuerpo
            }
        }
    }};
}

fn conectar(ui: &Ventana, est: &Est) {
    ui.global::<Util>()
        .on_formatear_monto(|t| mascara::formatear_entrada(&t).into());

    let app = ui.global::<App>();
    app.on_navegar(cb!(ui, est, |u, e, i| acciones::navegar(u, e, i)));
    app.on_deshacer(cb!(ui, est, |u, e, id| notificaciones::deshacer(u, e, id)));
    app.on_cerrar_aviso(cb!(ui, est, |u, e, id| notificaciones::quitar(u, e, id)));
    app.on_cambiar_tam(cb!(ui, est, |u, e, i| acciones::cambiar_tam(u, e, i)));
    app.on_ajustes_cambiados(con(ui, est, acciones::ajustes_cambiados));
    app.on_abrir_gasto(cb!(ui, est, |u, e, id| acciones::abrir_gasto(
        u,
        e,
        Some(i64::from(id)),
        None,
        None
    )));
    app.on_nuevo_gasto(con(ui, est, |ui, e| {
        acciones::abrir_gasto(ui, e, None, None, None)
    }));

    // Formularios.
    let f = ui.global::<Form>();
    f.on_guardar(con(ui, est, acciones::guardar));
    f.on_borrar(con(ui, est, acciones::borrar));
    f.on_cancelar(con(ui, est, acciones::cerrar_form));
    let cf = ui.global::<Confirmar>();
    cf.on_si(con(ui, est, acciones::confirmar_si));
    cf.on_no(con(ui, est, |ui, e| {
        ui.global::<Confirmar>().set_visible(false);
        e.borrow_mut().confirmar = None;
    }));

    // Tablero.
    let t = ui.global::<TableroD>();
    t.on_aplicar_rango(cb!(ui, est, |u, e, a, b| {
        e.borrow_mut().rango = Some(Rango {
            desde: de_clave(a),
            hasta: de_clave(b),
            nombre: None,
        });
        recargar(u, e);
    }));
    t.on_aplicar_preset(cb!(ui, est, |u, e, i| {
        let r = p_tablero::aplicar_preset(&mut e.borrow_mut(), i.max(0) as usize);
        if let Err(m) = r {
            notificaciones::error(u, e, &m);
        }
        recargar(u, e);
    }));
    t.on_aplicar_anio(cb!(ui, est, |u, e, a| {
        e.borrow_mut().rango = Some(Rango {
            desde: (a, 1),
            hasta: (a, 12),
            nombre: Some(format!("Año {a}")),
        });
        recargar(u, e);
    }));

    // Resumen.
    let r = ui.global::<ResumenD>();
    r.on_mover(cb!(ui, est, |u, e, delta| {
        {
            let mut x = e.borrow_mut();
            x.resumen = if x.modo_resumen == 1 {
                (x.resumen.0 + delta, x.resumen.1)
            } else {
                de_clave(clave(x.resumen) + delta)
            };
        }
        recargar(u, e);
    }));
    r.on_hoy(con(ui, est, |ui, e| {
        e.borrow_mut().resumen = hoy();
        recargar(ui, e);
    }));
    r.on_cambiar_modo(cb!(ui, est, |u, e, m| {
        e.borrow_mut().modo_resumen = m;
        recargar(u, e);
    }));
    r.on_abrir_mes_de_anio(cb!(ui, est, |u, e, i| {
        if i < 12 {
            let mut x = e.borrow_mut();
            x.resumen = (x.resumen.0, i as u32 + 1);
            x.modo_resumen = 0;
        }
        recargar(u, e);
    }));
    r.on_marcar_pagado(cb!(ui, est, |u, e, id| acciones::marcar_pagado(
        u,
        e,
        i64::from(id)
    )));
    r.on_editar_ingreso(cb!(ui, est, |u, e, id| acciones::abrir_ingreso(
        u,
        e,
        Some(i64::from(id))
    )));
    r.on_nuevo_ingreso(con(ui, est, |ui, e| acciones::abrir_ingreso(ui, e, None)));
    r.on_editar_mes(con(ui, est, acciones::abrir_mes));
    r.on_paginar(cb!(ui, est, |u, e, lista, sig| {
        {
            let mut x = e.borrow_mut();
            let (a, m) = x.resumen;
            let k = format!("{lista}-{a}-{m}");
            if lista == "gastos" {
                comun::mover_cursor(&mut x, &k, sig);
            } else {
                comun::mover_local(&mut x, &k, sig);
            }
        }
        recargar(u, e);
    }));

    // Movimientos.
    let m = ui.global::<MovsD>();
    m.on_filtrar(con(ui, est, |ui, e| {
        e.borrow_mut().cursores.remove("movimientos");
        recargar(ui, e);
    }));
    m.on_limpiar(con(ui, est, |ui, e| {
        let g = ui.global::<MovsD>();
        g.set_texto("".into());
        g.set_anio(0);
        g.set_mes(0);
        g.set_estado(0);
        g.set_pago(0);
        g.set_categoria(0);
        g.set_deuda(0);
        g.set_tarjeta(0);
        e.borrow_mut().cursores.remove("movimientos");
        recargar(ui, e);
    }));
    m.on_paginar(cb!(ui, est, |u, e, sig| {
        comun::mover_cursor(&mut e.borrow_mut(), "movimientos", sig);
        recargar(u, e);
    }));

    // Deudas.
    let dd = ui.global::<DeudasD>();
    dd.on_nueva(con(ui, est, |ui, e| acciones::abrir_deuda(ui, e, None)));
    dd.on_editar(cb!(ui, est, |u, e, id| acciones::abrir_deuda(
        u,
        e,
        Some(i64::from(id))
    )));
    dd.on_alternar(cb!(ui, est, |u, e, id| {
        {
            let mut x = e.borrow_mut();
            let id = i64::from(id);
            if !x.abiertas.remove(&id) {
                x.abiertas.insert(id);
            }
        }
        recargar(u, e);
    }));
    dd.on_nueva_cuota(cb!(ui, est, |u, e, id| acciones::abrir_gasto(
        u,
        e,
        None,
        Some(i64::from(id)),
        None
    )));
    dd.on_paginar_cuotas(cb!(ui, est, |u, e, id, sig| {
        comun::mover_cursor(&mut e.borrow_mut(), &format!("cuotas-{id}"), sig);
        recargar(u, e);
    }));
    dd.on_paginar(cb!(ui, est, |u, e, lista, sig| {
        comun::mover_local(&mut e.borrow_mut(), &format!("deudas-{lista}"), sig);
        recargar(u, e);
    }));

    // Tarjetas.
    let tt = ui.global::<TarjetasD>();
    tt.on_nueva(con(ui, est, |ui, e| acciones::abrir_tarjeta(ui, e, None)));
    tt.on_editar(cb!(ui, est, |u, e, id| acciones::abrir_tarjeta(
        u,
        e,
        Some(i64::from(id))
    )));
    tt.on_alternar(cb!(ui, est, |u, e, id| {
        {
            let mut x = e.borrow_mut();
            let id = i64::from(id);
            if !x.abiertas_tarjetas.remove(&id) {
                x.abiertas_tarjetas.insert(id);
            }
        }
        recargar(u, e);
    }));
    tt.on_nuevo_pago(cb!(ui, est, |u, e, id| acciones::abrir_gasto(
        u,
        e,
        None,
        None,
        Some(i64::from(id))
    )));
    tt.on_paginar_pagos(cb!(ui, est, |u, e, id, sig| {
        comun::mover_cursor(&mut e.borrow_mut(), &format!("pagos-{id}"), sig);
        recargar(u, e);
    }));
    tt.on_paginar(cb!(ui, est, |u, e, sig| {
        comun::mover_local(&mut e.borrow_mut(), "tarjetas", sig);
        recargar(u, e);
    }));

    // Catálogos.
    let cc = ui.global::<CatsD>();
    cc.on_agregar(cb!(ui, est, |u, e, tipo, nombre| acciones::agregar_item(
        u, e, tipo, &nombre
    )));
    cc.on_editar(cb!(ui, est, |u, e, tipo, id| acciones::abrir_item(
        u,
        e,
        tipo,
        i64::from(id)
    )));
    cc.on_paginar(cb!(ui, est, |u, e, tipo, sig| {
        comun::mover_local(&mut e.borrow_mut(), &format!("cat-{tipo}"), sig);
        recargar(u, e);
    }));

    // Importar y exportar.
    let dt = ui.global::<DatosD>();
    dt.on_importar(con(ui, est, |ui, e| {
        let forzar = ui.global::<DatosD>().get_forzar();
        acciones::importar(ui, e, forzar);
    }));
    dt.on_plantilla(con(ui, est, acciones::plantilla));
    dt.on_exportar_ods(con(ui, est, acciones::exportar_ods));
    dt.on_exportar_csv(con(ui, est, acciones::exportar_csv));
    dt.on_paginar(cb!(ui, est, |u, e, sig| {
        comun::mover_local(&mut e.borrow_mut(), "historial", sig);
        recargar(u, e);
    }));
}
