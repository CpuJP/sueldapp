//! Qué módulos muestra el Tablero, en qué orden y cómo. Se guarda en `tablero.txt` junto a la base.

use std::path::PathBuf;

pub const NOMBRES: [&str; 14] = [
    "Resumen del periodo",
    "Promedios y extremos",
    "Estado de los meses",
    "Ingresos y gastos por mes",
    "Lo que sobra o falta cada mes",
    "Saldo acumulado",
    "¿En qué se va el dinero?",
    "Gasto por categoría",
    "Estado de los pagos",
    "Conceptos con más gasto",
    "Deudas pendientes",
    "Gastos más grandes",
    "Ingresos por fuente",
    "Datos en tabla",
];
/// Ancho por defecto de cada módulo, en doceavos de la pantalla.
const ANCHOS_INICIALES: [u8; 14] = [12, 12, 12, 12, 6, 6, 4, 4, 4, 6, 6, 6, 6, 12];
pub const ANCHOS: [u8; 4] = [4, 6, 8, 12];
const HEROE: usize = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clase {
    Heroe,
    Cifras,
    Grafico,
    Grupos,
    Lineas,
    Tabla,
}

pub fn clase(tipo: usize) -> Clase {
    match tipo {
        HEROE => Clase::Heroe,
        1 | 2 => Clase::Cifras,
        3..=5 => Clase::Grafico,
        6..=9 | 12 => Clase::Grupos,
        10 | 11 => Clase::Lineas,
        _ => Clase::Tabla,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Modulo {
    pub tipo: usize,
    pub visible: bool,
    pub ancho: u8,
    /// 0 compacto, 1 normal, 2 grande.
    pub tam: u8,
    /// Grupos: 0 barras, 1 lista. Cifras: 0, 1 o 2 para 2, 3 o 4 columnas.
    pub vista: u8,
    /// Vacío: el título de `NOMBRES`.
    pub titulo: String,
}

impl Modulo {
    pub fn titulo(&self) -> &str {
        if self.titulo.is_empty() {
            NOMBRES[self.tipo]
        } else {
            &self.titulo
        }
    }

    /// El resumen no cabe en un tercio de pantalla.
    pub fn ancho_real(&self) -> u8 {
        if self.tipo == HEROE {
            self.ancho.max(6)
        } else {
            self.ancho
        }
    }

    /// Columnas de las cifras, limitadas por el ancho del módulo.
    pub fn columnas(&self) -> i32 {
        let pedidas = i32::from(self.vista) + 2;
        pedidas.min(match self.ancho_real() {
            ..=4 => 2,
            5..=6 => 3,
            _ => 4,
        })
    }

    /// Cuántos elementos muestran las listas; `None` es todos.
    pub fn limite(&self) -> Option<usize> {
        match self.tam {
            0 => Some(3),
            1 => Some(5),
            _ => None,
        }
    }

    pub fn alto_grafico(&self) -> f32 {
        [180.0, 260.0, 380.0][usize::from(self.tam.min(2))]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigTablero {
    pub modulos: Vec<Modulo>,
    /// 0 compacto, 1 normal, 2 amplio.
    pub espaciado: u8,
}

impl Default for ConfigTablero {
    fn default() -> Self {
        ConfigTablero {
            modulos: (0..NOMBRES.len())
                .map(|tipo| Modulo {
                    tipo,
                    visible: true,
                    ancho: ANCHOS_INICIALES[tipo],
                    // Como antes de poder configurarlo: gráficos normales, listas completas.
                    tam: if clase(tipo) == Clase::Grafico { 1 } else { 2 },
                    vista: if clase(tipo) == Clase::Cifras { 2 } else { 0 },
                    titulo: String::new(),
                })
                .collect(),
            espaciado: 1,
        }
    }
}

impl ConfigTablero {
    /// Lee el archivo; lo que no se entienda se ignora y los módulos que falten se añaden al final.
    pub fn desde_texto(texto: &str) -> Self {
        let base = Self::default();
        let mut c = ConfigTablero {
            modulos: Vec::new(),
            espaciado: base.espaciado,
        };
        for l in texto.lines() {
            if let Some(v) = l.strip_prefix("espaciado=") {
                c.espaciado = v.trim().parse().unwrap_or(1).min(2);
                continue;
            }
            let p: Vec<&str> = l.splitn(6, '|').collect();
            let [tipo, visible, ancho, tam, vista, titulo] = p[..] else {
                continue;
            };
            let Ok(tipo) = tipo.parse::<usize>() else {
                continue;
            };
            if tipo >= NOMBRES.len() || c.modulos.iter().any(|m| m.tipo == tipo) {
                continue;
            }
            let inicial = &base.modulos[tipo];
            c.modulos.push(Modulo {
                tipo,
                visible: visible != "0",
                ancho: ancho
                    .parse()
                    .ok()
                    .filter(|a| ANCHOS.contains(a))
                    .unwrap_or(inicial.ancho),
                tam: tam.parse().unwrap_or(inicial.tam).min(2),
                vista: vista.parse().unwrap_or(inicial.vista).min(2),
                titulo: titulo.to_string(),
            });
        }
        for m in base.modulos {
            if !c.modulos.iter().any(|x| x.tipo == m.tipo) {
                c.modulos.push(m);
            }
        }
        c
    }

    pub fn a_texto(&self) -> String {
        let mut t = format!("espaciado={}\n", self.espaciado);
        for m in &self.modulos {
            t += &format!(
                "{}|{}|{}|{}|{}|{}\n",
                m.tipo,
                u8::from(m.visible),
                m.ancho,
                m.tam,
                m.vista,
                m.titulo
            );
        }
        t
    }

    pub fn modulo(&self, tipo: usize) -> Option<&Modulo> {
        self.modulos.iter().find(|m| m.tipo == tipo)
    }

    pub fn modulo_mut(&mut self, tipo: usize) -> Option<&mut Modulo> {
        self.modulos.iter_mut().find(|m| m.tipo == tipo)
    }

    pub fn fijar_titulo(&mut self, tipo: usize, titulo: &str) {
        if let Some(m) = self.modulo_mut(tipo) {
            // Una línea por módulo en el archivo.
            m.titulo = titulo.replace(['\n', '\r'], " ").trim().to_string();
        }
    }

    /// Intercambia un módulo con el visible anterior (`atras`) o el siguiente.
    pub fn mover(&mut self, tipo: usize, atras: bool) {
        let visibles: Vec<usize> = (0..self.modulos.len())
            .filter(|i| self.modulos[*i].visible)
            .collect();
        let Some(p) = visibles.iter().position(|i| self.modulos[*i].tipo == tipo) else {
            return;
        };
        let otro = if atras { p.checked_sub(1) } else { Some(p + 1) };
        if let Some(o) = otro.and_then(|o| visibles.get(o)) {
            self.modulos.swap(visibles[p], *o);
        }
    }

    /// Coloca un módulo justo antes o después de otro; así funciona arrastrar y soltar.
    pub fn mover_a(&mut self, tipo: usize, destino: usize, antes: bool) {
        if tipo == destino || self.modulo(destino).is_none() {
            return;
        }
        let Some(origen) = self.modulos.iter().position(|m| m.tipo == tipo) else {
            return;
        };
        let m = self.modulos.remove(origen);
        let d = self
            .modulos
            .iter()
            .position(|x| x.tipo == destino)
            .unwrap_or(self.modulos.len());
        self.modulos.insert(if antes { d } else { d + 1 }, m);
    }

    /// Reparte los módulos visibles en filas de hasta 12 doceavos.
    pub fn filas(&self) -> Vec<Vec<&Modulo>> {
        let mut filas: Vec<Vec<&Modulo>> = Vec::new();
        let mut usado = 0;
        for m in self.modulos.iter().filter(|m| m.visible) {
            let a = m.ancho_real();
            match filas.last_mut() {
                Some(f) if usado + a <= 12 => f.push(m),
                _ => {
                    filas.push(vec![m]);
                    usado = 0;
                }
            }
            usado += a;
        }
        filas
    }
}

fn ruta() -> PathBuf {
    sueldapp_almacen::ruta_por_defecto().with_file_name("tablero.txt")
}

pub fn leer() -> ConfigTablero {
    std::fs::read_to_string(ruta())
        .map(|t| ConfigTablero::desde_texto(&t))
        .unwrap_or_default()
}

pub fn guardar(c: &ConfigTablero) {
    // Si no se puede escribir, la configuración dura solo esta sesión.
    let _ = std::fs::write(ruta(), c.a_texto());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tipos(c: &ConfigTablero) -> Vec<Vec<usize>> {
        c.filas()
            .iter()
            .map(|f| f.iter().map(|m| m.tipo).collect())
            .collect()
    }

    #[test]
    fn por_defecto_conserva_el_tablero_de_siempre() {
        let c = ConfigTablero::default();
        assert_eq!(
            tipos(&c),
            [
                vec![0],
                vec![1],
                vec![2],
                vec![3],
                vec![4, 5],
                vec![6, 7, 8],
                vec![9, 10],
                vec![11, 12],
                vec![13]
            ]
        );
        assert_eq!(c.modulo(1).unwrap().columnas(), 4);
        assert_eq!(c.modulo(6).unwrap().limite(), None);
        assert_eq!(c.modulo(3).unwrap().alto_grafico(), 260.0);
    }

    #[test]
    fn ida_y_vuelta_y_archivos_raros() {
        let mut c = ConfigTablero {
            espaciado: 2,
            ..Default::default()
        };
        c.mover(4, true);
        c.modulo_mut(6).unwrap().visible = false;
        c.modulo_mut(7).unwrap().ancho = 8;
        c.fijar_titulo(3, "Mis | cuentas\nde hoy");
        assert_eq!(c.modulo(3).unwrap().titulo(), "Mis | cuentas de hoy");
        assert_eq!(ConfigTablero::desde_texto(&c.a_texto()), c);

        assert_eq!(ConfigTablero::desde_texto(""), ConfigTablero::default());
        let raro = ConfigTablero::desde_texto(
            "espaciado=99\nbasura\n99|1|6|1|0|x\n5|0|7|9|9|Saldo\n5|1|6|1|0|repetido\n",
        );
        assert_eq!(raro.espaciado, 2);
        assert_eq!(raro.modulos.len(), NOMBRES.len());
        // El guardado va primero; lo inválido toma el valor inicial o el tope.
        let m = &raro.modulos[0];
        assert_eq!(
            (m.tipo, m.visible, m.ancho, m.tam, m.vista),
            (5, false, 6, 2, 2)
        );
        assert_eq!(m.titulo(), "Saldo");
    }

    #[test]
    fn mover_salta_los_ocultos_y_respeta_los_bordes() {
        let mut c = ConfigTablero::default();
        c.modulo_mut(1).unwrap().visible = false;
        c.mover(2, true);
        assert_eq!(
            c.modulos.iter().take(3).map(|m| m.tipo).collect::<Vec<_>>(),
            [2, 1, 0]
        );
        let antes = c.clone();
        c.mover(2, true);
        c.mover(13, false);
        c.mover(1, true);
        assert_eq!(c, antes);
    }

    #[test]
    fn arrastrar_coloca_antes_o_despues() {
        let orden = |c: &ConfigTablero| -> Vec<usize> {
            c.modulos.iter().take(4).map(|m| m.tipo).collect()
        };
        let mut c = ConfigTablero::default();
        c.mover_a(0, 2, false);
        assert_eq!(orden(&c), [1, 2, 0, 3]);
        c.mover_a(0, 1, true);
        assert_eq!(orden(&c), [0, 1, 2, 3]);
        c.mover_a(3, 0, true);
        assert_eq!(orden(&c), [3, 0, 1, 2]);
        let antes = c.clone();
        c.mover_a(1, 1, true);
        c.mover_a(1, 99, true);
        c.mover_a(99, 1, true);
        assert_eq!(c, antes);
        assert_eq!(c.modulos.len(), NOMBRES.len());
    }

    #[test]
    fn filas_y_limites() {
        let mut c = ConfigTablero::default();
        for m in &mut c.modulos {
            m.visible = false;
        }
        assert!(c.filas().is_empty());
        for (tipo, ancho) in [(3, 8), (4, 6), (5, 6), (1, 4), (6, 12)] {
            let m = c.modulo_mut(tipo).unwrap();
            m.visible = true;
            m.ancho = ancho;
        }
        assert_eq!(tipos(&c), [vec![1, 3], vec![4, 5], vec![6]]);

        let m = c.modulo_mut(0).unwrap();
        m.ancho = 4;
        assert_eq!(m.ancho_real(), 6);
        let m = c.modulo_mut(1).unwrap();
        m.vista = 2;
        assert_eq!(m.columnas(), 2);
        m.ancho = 6;
        assert_eq!(m.columnas(), 3);
        m.ancho = 12;
        assert_eq!(m.columnas(), 4);
        m.vista = 0;
        assert_eq!(m.columnas(), 2);
        let m = c.modulo_mut(6).unwrap();
        m.tam = 0;
        assert_eq!(m.limite(), Some(3));
        m.tam = 1;
        assert_eq!(m.limite(), Some(5));
    }
}
