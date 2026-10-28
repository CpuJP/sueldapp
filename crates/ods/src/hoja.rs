//! Extrae la estructura de la pestaña `Nómina` (ver `docs/analisis-hoja.md`):
//! zona de deudas (A8:E…) y franjas anuales de bloques mensuales (F, K, P, U…).

use crate::lectura::{ref_celda, Celda, Hoja};
use crate::Error;
use rust_decimal::prelude::ToPrimitive;
use std::collections::BTreeSet;
use std::str::FromStr;
use sueldapp_dominio::calculos::redondear;
use sueldapp_dominio::texto::normalizar;
use sueldapp_dominio::{Decimal, NOMBRES_MES};

/// Un número de la hoja con su celda de origen.
#[derive(Debug, Clone, PartialEq)]
pub struct Valor {
    pub fila: u32,
    pub col: u32,
    /// Redondeado a 2 decimales.
    pub valor: Decimal,
    /// `office:value` original.
    pub crudo: String,
    pub formula: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GastoHoja {
    pub concepto: String,
    pub monto: Valor,
    pub estado: String,
    pub fuera_de_bloque: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BloqueHoja {
    pub anio: i32,
    pub mes: u32,
    pub encabezado: String,
    pub fila: u32,
    pub cerrado: bool,
    pub gastos: Vec<GastoHoja>,
    pub sueldo: Option<Valor>,
    pub prima: Option<Valor>,
    pub total: Option<Decimal>,
    pub subtotal: Option<Decimal>,
    pub restante: Option<Decimal>,
    pub subrestante: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnioHoja {
    pub anio: i32,
    pub pago: Option<Decimal>,
    pub gasto: Option<Decimal>,
    pub restante: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeudaHoja {
    pub fila: u32,
    pub nombre: String,
    pub pago: Option<Valor>,
    pub total: Option<Valor>,
    pub pendiente: Option<Decimal>,
    pub saldada: bool,
}

/// Aviso para el log: celda y detalle.
#[derive(Debug, Clone, PartialEq)]
pub struct Aviso {
    pub celda: String,
    pub detalle: String,
    /// `true` si la fila no se importa.
    pub rechazada: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Nomina {
    pub bloques: Vec<BloqueHoja>,
    pub anios: Vec<AnioHoja>,
    pub deudas: Vec<DeudaHoja>,
    /// Transformaciones y rechazos detectados al leer.
    pub avisos: Vec<Aviso>,
    /// Celdas no usadas como dato (títulos, encabezados).
    pub omitidas: Vec<Aviso>,
}

/// Fila (desde 0) de los años de cada franja: F4, K4, P4, U4.
const FILA_ANIOS: u32 = 3;
/// Primera fila de deudas (A8).
const FILA_DEUDAS: u32 = 7;

fn decimal(c: &Celda) -> Option<(Decimal, String)> {
    let crudo = c.valor.as_ref()?;
    let d = Decimal::from_str(crudo)
        .or_else(|_| Decimal::from_scientific(crudo))
        .ok()?;
    Some((d, crudo.clone()))
}

struct Lector<'a> {
    hoja: &'a Hoja,
    usadas: BTreeSet<(u32, u32)>,
    avisos: Vec<Aviso>,
}

impl<'a> Lector<'a> {
    fn texto(&mut self, f: u32, c: u32) -> String {
        let t = self.hoja.texto(f, c).to_string();
        if !t.is_empty() {
            self.usadas.insert((f, c));
        }
        t
    }

    fn valor(&mut self, f: u32, c: u32) -> Option<Valor> {
        let celda = self.hoja.celda(f, c)?;
        let (d, crudo) = decimal(celda)?;
        self.usadas.insert((f, c));
        let valor = redondear(d);
        if valor != d {
            self.avisos.push(Aviso {
                celda: ref_celda(f, c),
                detalle: format!("valor {crudo} redondeado a {valor}"),
                rechazada: false,
            });
        }
        Some(Valor {
            fila: f,
            col: c,
            valor,
            crudo,
            formula: celda.formula.clone(),
        })
    }

    fn numero(&mut self, f: u32, c: u32) -> Option<Decimal> {
        self.valor(f, c).map(|v| v.valor)
    }

    fn aviso(&mut self, f: u32, c: u32, detalle: impl Into<String>) {
        self.avisos.push(Aviso {
            celda: ref_celda(f, c),
            detalle: detalle.into(),
            rechazada: false,
        });
    }

    fn rechazo(&mut self, f: u32, c: u32, detalle: impl Into<String>) {
        self.avisos.push(Aviso {
            celda: ref_celda(f, c),
            detalle: detalle.into(),
            rechazada: true,
        });
    }
}

fn mes_de(palabra: &str) -> Option<u32> {
    let p = normalizar(palabra);
    NOMBRES_MES
        .iter()
        .position(|m| normalizar(m) == p)
        .map(|i| i as u32 + 1)
}

/// Lee la pestaña `Nómina`.
pub fn extraer(hoja: &Hoja) -> Result<Nomina, Error> {
    let mut l = Lector {
        hoja,
        usadas: BTreeSet::new(),
        avisos: Vec::new(),
    };
    let franjas: Vec<(u32, i32)> = hoja
        .celdas
        .range((FILA_ANIOS, 0)..(FILA_ANIOS + 1, 0))
        .filter_map(|(&(_, c), celda)| {
            let (d, _) = decimal(celda)?;
            let anio = d.to_i32()?;
            (2000..=2100).contains(&anio).then_some((c, anio))
        })
        .collect();
    if franjas.is_empty() {
        return Err(Error::Formato(
            "La pestaña Nómina no tiene la fila de años (F4, K4…). No parece la hoja original."
                .into(),
        ));
    }
    for &(c, _) in &franjas {
        l.usadas.insert((FILA_ANIOS, c));
    }

    let mut n = Nomina {
        deudas: extraer_deudas(&mut l),
        ..Default::default()
    };
    for &(c0, anio) in &franjas {
        extraer_franja(&mut l, c0, anio, &mut n);
    }

    n.omitidas = hoja
        .celdas
        .iter()
        .filter(|(k, c)| !l.usadas.contains(k) && !c.vacia())
        .map(|(&(f, c), celda)| Aviso {
            celda: ref_celda(f, c),
            detalle: format!("título o encabezado sin dato: \"{}\"", celda.texto.trim()),
            rechazada: false,
        })
        .collect();
    n.avisos = l.avisos;
    Ok(n)
}

fn extraer_deudas(l: &mut Lector) -> Vec<DeudaHoja> {
    // Encabezados Item | Pago | Total | Pendiente en la fila anterior.
    for c in 0..4 {
        l.texto(FILA_DEUDAS - 1, c);
    }
    let mut v = Vec::new();
    let mut f = FILA_DEUDAS;
    loop {
        let nombre = l.texto(f, 0);
        if nombre.is_empty() {
            break;
        }
        v.push(DeudaHoja {
            fila: f,
            nombre,
            pago: l.valor(f, 1),
            total: l.valor(f, 2),
            pendiente: l.numero(f, 3),
            saldada: l.texto(f, 4).eq_ignore_ascii_case("ok"),
        });
        f += 1;
    }
    v
}

#[derive(PartialEq)]
enum Zona {
    Antes,
    Dentro,
    Fuera,
}

fn extraer_franja(l: &mut Lector, c0: u32, anio: i32, n: &mut Nomina) {
    let filas: Vec<u32> = l
        .hoja
        .celdas
        .keys()
        .filter(|&&(f, c)| c == c0 && f > FILA_ANIOS)
        .map(|&(f, _)| f)
        .collect();
    let mut bloques: Vec<BloqueHoja> = Vec::new();
    let mut huerfanos: Vec<GastoHoja> = Vec::new();
    let mut zona = Zona::Antes;
    let mut resumen = AnioHoja {
        anio,
        ..Default::default()
    };

    for f in filas {
        let t = l.texto(f, c0);
        if t.is_empty() {
            continue;
        }
        let titulo_monto = l.hoja.texto(f, c0 + 1) == "Gastos fijos";
        let palabras: Vec<&str> = t.split_whitespace().collect();
        let mes = (palabras.len() >= 2 && palabras[0] == "Sueldo")
            .then(|| mes_de(palabras[1]))
            .flatten();

        if titulo_monto {
            for c in c0..c0 + 3 {
                l.texto(f, c);
            }
            if t != "Concepto" {
                l.aviso(
                    f,
                    c0,
                    format!("título \"{t}\" en lugar de \"Concepto\"; se ignora"),
                );
            }
            continue;
        }
        if let Some(mes) = mes {
            if let Some(escrito) = palabras.get(2).and_then(|a| a.parse::<i32>().ok()) {
                if escrito != anio {
                    l.aviso(
                        f,
                        c0,
                        format!("el encabezado dice {escrito} pero la columna es de {anio}; se usa {anio}"),
                    );
                }
            }
            let cerrado = l.texto(f, c0 + 2).eq_ignore_ascii_case("ok");
            let mut b = BloqueHoja {
                anio,
                mes,
                encabezado: t,
                fila: f,
                cerrado,
                ..Default::default()
            };
            // Gastos sueltos antes de este encabezado: pertenecen a este mes.
            b.gastos.append(&mut huerfanos);
            bloques.push(b);
            zona = Zona::Dentro;
            continue;
        }
        let Some(b) = bloques.last_mut() else {
            l.rechazo(f, c0, format!("\"{t}\" está antes del primer mes"));
            continue;
        };
        match t.as_str() {
            "Total" => {
                b.total = l.numero(f, c0 + 1);
                l.texto(f, c0 + 2);
                b.subtotal = l.numero(f, c0 + 3);
                zona = Zona::Fuera;
            }
            "Sueldo" => b.sueldo = l.valor(f, c0 + 1),
            "Prima" => b.prima = l.valor(f, c0 + 1),
            "Restante" => {
                b.restante = l.numero(f, c0 + 1);
                l.texto(f, c0 + 2);
                b.subrestante = l.numero(f, c0 + 3);
            }
            _ if t.starts_with("TOTAL PAGO") => resumen.pago = l.numero(f, c0 + 1),
            _ if t.starts_with("TOTAL GASTO") => resumen.gasto = l.numero(f, c0 + 1),
            _ if t.starts_with("TOTAL RESTANTE") => resumen.restante = l.numero(f, c0 + 1),
            _ => {
                let Some(monto) = l.valor(f, c0 + 1) else {
                    l.rechazo(f, c0, format!("\"{t}\" no tiene monto"));
                    continue;
                };
                let estado = l.texto(f, c0 + 2);
                let g = GastoHoja {
                    concepto: t,
                    monto,
                    estado,
                    fuera_de_bloque: zona != Zona::Dentro,
                };
                if zona == Zona::Dentro {
                    b.gastos.push(g);
                } else {
                    huerfanos.push(g);
                }
            }
        }
    }
    // Gastos sueltos al final de la franja: al último mes.
    if let Some(b) = bloques.last_mut() {
        b.gastos.append(&mut huerfanos);
    }
    n.bloques.append(&mut bloques);
    n.anios.push(resumen);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meses() {
        assert_eq!(mes_de("Septiembre"), Some(9));
        assert_eq!(mes_de("febrero"), Some(2));
        assert_eq!(mes_de("Total"), None);
    }
}
