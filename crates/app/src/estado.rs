//! Estado de la app entre pantallas: conexión, periodo del tablero, páginas, deshacer y ajustes.

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

/// Mes actual (UTC; en Colombia solo difiere las últimas 5 horas del último día del mes).
pub fn hoy() -> Periodo {
    let dias = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    // Algoritmo civil de Howard Hinnant.
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let anio = yoe + era * 400 + i64::from(mes <= 2);
    (anio as i32, mes as u32)
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
    };
    if let Ok(t) = std::fs::read_to_string(ruta_ajustes()) {
        for l in t.lines() {
            if let Some((k, v)) = l.split_once('=') {
                match k.trim() {
                    "tema" => a.tema = v.trim().parse().unwrap_or(0),
                    "solido" => a.solido = v.trim() == "1",
                    "sin_movimiento" => a.sin_movimiento = v.trim() == "1",
                    "tam" => a.tam = v.trim().parse().unwrap_or(0),
                    _ => {}
                }
            }
        }
    }
    a
}

pub fn guardar_ajustes(a: &Ajustes) {
    let t = format!(
        "tema={}\nsolido={}\nsin_movimiento={}\ntam={}\n",
        a.tema,
        u8::from(a.solido),
        u8::from(a.sin_movimiento),
        a.tam
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
