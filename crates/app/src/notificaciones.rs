//! Todas las notificaciones de la app. Aquí se decide el tipo (franja verde, naranja o roja),
//! cuánto duran y qué acción de deshacer llevan; las pantallas solo llaman a estas funciones.

use crate::acciones::{recargar, Est};
use crate::comun::{modelo, s};
use crate::estado::Deshacer;
use crate::{App, Aviso, Ventana};
use slint::ComponentHandle;
use std::time::Duration;

/// Tipo de notificación; el número es el que entiende `notificaciones.slint`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    /// Algo se creó, guardó o terminó bien (verde).
    Exito = 0,
    /// Algo se borró, se deshizo o merece atención (naranja).
    Advertencia = 1,
    /// Un error o algo crítico (rojo).
    Error = 2,
}

/// Segundos en pantalla: más tiempo si hay que leer un error o decidir si deshacer.
fn duracion(tipo: Tipo, con_deshacer: bool) -> u64 {
    match (tipo, con_deshacer) {
        (_, true) => 9,
        (Tipo::Error, false) => 8,
        _ => 5,
    }
}

pub fn notificar(ui: &Ventana, est: &Est, tipo: Tipo, texto: &str, deshacer: Option<Deshacer>) {
    let con_deshacer = deshacer.is_some();
    let id = {
        let mut e = est.borrow_mut();
        let id = e.sig_aviso;
        e.sig_aviso += 1;
        if let Some(d) = deshacer {
            e.deshacer.insert(id, d);
        }
        e.avisos.push(Aviso {
            id,
            texto: s(texto),
            tipo: tipo as i32,
            deshacer: con_deshacer,
        });
        // Como mucho 4 a la vez: la más vieja se va (y con ella su deshacer).
        while e.avisos.len() > 4 {
            let viejo = e.avisos.remove(0).id;
            e.deshacer.remove(&viejo);
        }
        ui.global::<App>().set_avisos(modelo(e.avisos.clone()));
        id
    };
    let (debil, est2) = (ui.as_weak(), est.clone());
    slint::Timer::single_shot(
        Duration::from_secs(duracion(tipo, con_deshacer)),
        move || {
            if let Some(ui) = debil.upgrade() {
                quitar(&ui, &est2, id);
            }
        },
    );
}

pub fn exito(ui: &Ventana, est: &Est, texto: &str) {
    notificar(ui, est, Tipo::Exito, texto, None);
}

pub fn advertencia(ui: &Ventana, est: &Est, texto: &str) {
    notificar(ui, est, Tipo::Advertencia, texto, None);
}

pub fn error(ui: &Ventana, est: &Est, texto: &str) {
    notificar(ui, est, Tipo::Error, texto, None);
}

pub fn quitar(ui: &Ventana, est: &Est, id: i32) {
    let mut e = est.borrow_mut();
    e.avisos.retain(|a| a.id != id);
    e.deshacer.remove(&id);
    ui.global::<App>().set_avisos(modelo(e.avisos.clone()));
}

/// Ejecuta el deshacer de la notificación `id` y avisa del resultado.
pub fn deshacer(ui: &Ventana, est: &Est, id: i32) {
    let d = est.borrow_mut().deshacer.remove(&id);
    quitar(ui, est, id);
    if let Some(d) = d {
        let r = d(&est.borrow().c);
        match r {
            Ok(()) => advertencia(ui, est, "Cambio deshecho."),
            Err(m) => error(ui, est, &m),
        }
        recargar(ui, est);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duraciones() {
        assert_eq!(duracion(Tipo::Exito, false), 5);
        assert_eq!(duracion(Tipo::Error, false), 8);
        assert_eq!(duracion(Tipo::Advertencia, true), 9);
        assert_eq!(Tipo::Error as i32, 2);
    }
}
