//! Estado de la app entre pantallas: conexión, periodo del tablero, páginas, deshacer y ajustes.

use chrono::Datelike;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use sueldapp_almacen::Connection;
use sueldapp_dominio::paginacion::Direccion;
use sueldapp_dominio::{Gasto, Periodo};

pub type Deshacer = Box<dyn FnOnce(&Connection) -> Result<(), String>>;
/// Acción confirmada por la persona: devuelve el aviso y cómo deshacerla.
pub type AccionConfirmada = Box<dyn FnOnce(&mut Estado) -> Result<(String, Deshacer), String>>;

/// Qué registro está abierto en el formulario y cómo traducir sus selectores a ids.
pub enum Formulario {
    Gasto {
        id: Option<i64>,
        estados: Vec<i64>,
        categorias: Vec<Option<i64>>,
        deudas: Vec<Option<i64>>,
        tarjetas: Vec<Option<i64>>,
    },
    Ingreso {
        id: Option<i64>,
        fuentes: Vec<i64>,
    },
    Deuda {
        id: Option<i64>,
    },
    Tarjeta {
        id: Option<i64>,
    },
    Mes {
        mes: sueldapp_dominio::Mes,
        nombre: String,
    },
    Item {
        tipo: i32, // 0 estados, 1 categorías, 2 fuentes
        id: i64,
        orden: i64,
        reasignar: Vec<Option<i64>>,
    },
}

pub struct Rango {
    pub desde: Periodo,
    pub hasta: Periodo,
    pub nombre: Option<String>,
}

pub struct Estado {
    pub c: Connection,
    pub rango: Option<Rango>,
    pub resumen: Periodo,
    pub modo_resumen: i32,
    pub cursores: HashMap<String, (Option<String>, Direccion)>,
    pub ultimas: HashMap<String, (Option<String>, Option<String>)>,
    pub inicios: HashMap<String, usize>,
    pub abiertas: HashSet<i64>,
    pub abiertas_tarjetas: HashSet<i64>,
    pub visibles: HashMap<i64, Gasto>,
    pub ingresos_visibles: HashMap<i64, sueldapp_dominio::Ingreso>,
    pub form: Option<Formulario>,
    pub confirmar: Option<AccionConfirmada>,
    /// Si la acción por confirmar es buena noticia (aviso verde) en vez de un borrado.
    pub confirmar_exito: bool,
    /// La acción por confirmar no se puede deshacer.
    pub confirmar_sin_deshacer: bool,
    pub tablero: crate::tablero_config::ConfigTablero,
    /// Meses completos en la última recarga; `None` antes de la primera.
    pub meses_completos: Option<HashSet<i64>>,
    /// Alertas ya avisadas: (gasto, vencido). Vencer vuelve a avisar una vez.
    pub alertas_avisadas: HashSet<(i64, bool)>,
    pub deshacer: HashMap<i32, Deshacer>,
    pub avisos: Vec<crate::Aviso>,
    pub sig_aviso: i32,
    pub tam_indice: i32,
    pub ultimo_informe: Option<sueldapp_ods::importar::Informe>,
}

impl Estado {
    pub fn nuevo(c: Connection) -> Estado {
        Estado {
            c,
            rango: None,
            resumen: hoy(),
            modo_resumen: 0,
            cursores: HashMap::new(),
            ultimas: HashMap::new(),
            inicios: HashMap::new(),
            abiertas: HashSet::new(),
            abiertas_tarjetas: HashSet::new(),
            visibles: HashMap::new(),
            ingresos_visibles: HashMap::new(),
            form: None,
            confirmar: None,
            confirmar_exito: false,
            confirmar_sin_deshacer: false,
            tablero: crate::tablero_config::leer(),
            meses_completos: None,
            alertas_avisadas: HashSet::new(),
            deshacer: HashMap::new(),
            avisos: Vec::new(),
            sig_aviso: 1,
            tam_indice: 0,
            ultimo_informe: None,
        }
    }

    pub fn tam(&self) -> usize {
        [10, 25, 50][self.tam_indice.clamp(0, 2) as usize]
    }
}

/// Mes actual en la hora local del equipo.
pub fn hoy() -> Periodo {
    let h = chrono::Local::now().date_naive();
    (h.year(), h.month())
}

/// Fecha actual en la hora local del equipo, `AAAA-MM-DD`.
pub fn hoy_iso() -> String {
    chrono::Local::now().date_naive().to_string()
}

pub fn clave(p: Periodo) -> i32 {
    p.0 * 12 + p.1 as i32 - 1
}

pub fn de_clave(k: i32) -> Periodo {
    (k.div_euclid(12), (k.rem_euclid(12) + 1) as u32)
}

/// Ajustes de apariencia en `ajustes.txt` junto a la base (clave=valor).
pub struct Ajustes {
    pub tema: i32,
    pub solido: bool,
    pub sin_movimiento: bool,
    pub tam: i32,
    pub alertas_activas: bool,
    pub alertas_dias: i64,
    pub alertas_vencidos: bool,
    pub alertas_emergente: bool,
    pub alertas_boton: bool,
}

fn ruta_ajustes() -> PathBuf {
    sueldapp_almacen::ruta_por_defecto().with_file_name("ajustes.txt")
}

pub fn leer_ajustes() -> Ajustes {
    let mut a = Ajustes {
        tema: 0,
        solido: false,
        sin_movimiento: false,
        tam: 0,
        alertas_activas: true,
        alertas_dias: 7,
        alertas_vencidos: true,
        alertas_emergente: true,
        alertas_boton: true,
    };
    if let Ok(t) = std::fs::read_to_string(ruta_ajustes()) {
        for l in t.lines() {
            if let Some((k, v)) = l.split_once('=') {
                match k.trim() {
                    "tema" => a.tema = v.trim().parse().unwrap_or(0),
                    "solido" => a.solido = v.trim() == "1",
                    "sin_movimiento" => a.sin_movimiento = v.trim() == "1",
                    "tam" => a.tam = v.trim().parse().unwrap_or(0),
                    "alertas_activas" => a.alertas_activas = v.trim() != "0",
                    "alertas_dias" => a.alertas_dias = v.trim().parse().unwrap_or(7),
                    "alertas_vencidos" => a.alertas_vencidos = v.trim() != "0",
                    "alertas_emergente" => a.alertas_emergente = v.trim() != "0",
                    "alertas_boton" => a.alertas_boton = v.trim() != "0",
                    _ => {}
                }
            }
        }
    }
    a
}

pub fn guardar_ajustes(a: &Ajustes) {
    let t = format!(
        "tema={}\nsolido={}\nsin_movimiento={}\ntam={}\nalertas_activas={}\nalertas_dias={}\n\
         alertas_vencidos={}\nalertas_emergente={}\nalertas_boton={}\n",
        a.tema,
        u8::from(a.solido),
        u8::from(a.sin_movimiento),
        a.tam,
        u8::from(a.alertas_activas),
        a.alertas_dias,
        u8::from(a.alertas_vencidos),
        u8::from(a.alertas_emergente),
        u8::from(a.alertas_boton)
    );
    // Si no se puede escribir, los ajustes duran solo esta sesión.
    let _ = std::fs::write(ruta_ajustes(), t);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claves_de_mes() {
        assert_eq!(de_clave(clave((2026, 10))), (2026, 10));
        assert_eq!(de_clave(clave((2026, 1)) - 1), (2025, 12));
        let (a, m) = hoy();
        assert!(a >= 2024 && (1..=12).contains(&m));
    }
}
