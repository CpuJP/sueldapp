//! Plantilla para llenar a mano: un `.ods` con hojas simples (Gastos, Ingresos, Deudas,
//! Tarjetas) que se puede llenar en cualquier hoja de cálculo y luego importar. A diferencia de
//! la exportación propia, la plantilla *suma* datos: no reemplaza lo que ya hay.
//!
//! Idempotencia: cada fila se guarda con un `origen` derivado de su contenido (más un contador
//! para filas idénticas), así que importar el mismo contenido dos veces no duplica nada.

use crate::conciliar::{Diferencia, Resultado};
use crate::escritura::{Dato, Tabla};
use crate::importar::{Accion, Conteos, Informe, LineaLog};
use crate::lectura::{Celda, Hoja};
use crate::Error;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::str::FromStr;
use sueldapp_almacen::repo::{self, Catalogo};
use sueldapp_dominio::calculos::redondear;
use sueldapp_dominio::formato::leer_monto;
use sueldapp_dominio::texto::normalizar;
use sueldapp_dominio::{Decimal, Deuda, Gasto, Ingreso, Mes, Tarjeta, NOMBRES_MES};

const INSTRUCCIONES: &str = "Instrucciones";

const COLS_GASTOS: [&str; 9] = [
    "Año",
    "Mes",
    "Concepto",
    "Monto",
    "Estado",
    "Categoría",
    "Deuda",
    "Tarjeta",
    "Nota",
];
const COLS_INGRESOS: [&str; 5] = ["Año", "Mes", "Fuente", "Monto", "Nota"];
const COLS_DEUDAS: [&str; 3] = ["Nombre", "Total", "Notas"];
const COLS_TARJETAS: [&str; 5] = ["Nombre", "Cupo", "Saldo", "Día de corte", "Día de pago"];

fn texto(t: &str) -> Dato {
    Dato::Texto(t.to_string())
}

/// Hojas de la plantilla. `Listas` muestra los nombres válidos que ya existen en la app.
pub fn tablas_plantilla(c: &Connection) -> Result<Vec<Tabla>, Error> {
    let instrucciones = [
        "Plantilla de Sueldapp para llenar a mano.",
        "",
        "1. Llena las hojas Gastos, Ingresos, Deudas y Tarjetas. Una fila por registro; deja la fila de títulos.",
        "2. Año: 4 cifras (2026). Mes: número (1-12) o nombre (Octubre).",
        "3. Montos en pesos: 150000, 150.000 o 150000,50. Máximo 2 decimales.",
        "4. Estado: Pagado, Pendiente, No, Aplazado o Sin estado (vacío = Pendiente). Mira la hoja Listas.",
        "5. Categoría, Deuda, Tarjeta y Fuente: escribe el nombre. Si no existe, la app lo crea.",
        "6. En la app: Importar y exportar > Elegir archivo e importar.",
        "",
        "Si alguna fila tiene un error, no se guarda nada y el informe dice qué fila corregir.",
        "Importar el mismo contenido dos veces no duplica datos. La plantilla suma datos; no borra nada.",
    ];
    let lista = |v: Vec<String>| v;
    let estados = lista(
        repo::listar_estados(c)?
            .into_iter()
            .map(|e| e.nombre)
            .collect(),
    );
    let categorias = lista(
        repo::listar_items(c, Catalogo::Categorias)?
            .into_iter()
            .map(|e| e.nombre)
            .collect(),
    );
    let fuentes = lista(
        repo::listar_items(c, Catalogo::Fuentes)?
            .into_iter()
            .map(|e| e.nombre)
            .collect(),
    );
    let deudas = lista(
        repo::listar_deudas(c)?
            .into_iter()
            .map(|e| e.nombre)
            .collect(),
    );
    let tarjetas = lista(
        repo::listar_tarjetas(c)?
            .into_iter()
            .map(|e| e.nombre)
            .collect(),
    );
    let columnas = [&estados, &categorias, &fuentes, &deudas, &tarjetas];
    let filas_listas = (0..columnas.iter().map(|c| c.len()).max().unwrap_or(0).max(12))
        .map(|i| {
            let mut f: Vec<Dato> = columnas
                .iter()
                .map(|c| c.get(i).map(|s| texto(s)).unwrap_or(Dato::Vacio))
                .collect();
            f.push(NOMBRES_MES.get(i).map(|m| texto(m)).unwrap_or(Dato::Vacio));
            f
        })
        .collect();

    Ok(vec![
        Tabla {
            nombre: INSTRUCCIONES,
            encabezados: vec!["Cómo usar esta plantilla"],
            filas: instrucciones.iter().map(|t| vec![texto(t)]).collect(),
        },
        Tabla {
            nombre: "Gastos",
            encabezados: COLS_GASTOS.to_vec(),
            filas: vec![],
        },
        Tabla {
            nombre: "Ingresos",
            encabezados: COLS_INGRESOS.to_vec(),
            filas: vec![],
        },
        Tabla {
            nombre: "Deudas",
            encabezados: COLS_DEUDAS.to_vec(),
            filas: vec![],
        },
        Tabla {
            nombre: "Tarjetas",
            encabezados: COLS_TARJETAS.to_vec(),
            filas: vec![],
        },
        Tabla {
            nombre: "Listas",
            encabezados: vec![
                "Estados",
                "Categorías",
                "Fuentes",
                "Deudas",
                "Tarjetas",
                "Meses",
            ],
            filas: filas_listas,
        },
    ])
}

pub fn es_plantilla(hojas: &[Hoja]) -> bool {
    hojas.iter().any(|h| h.nombre == INSTRUCCIONES) && hojas.iter().any(|h| h.nombre == "Gastos")
}

/// Fila de datos: número de fila (desde 1, como en la hoja) y celdas por encabezado.
struct Fila<'a> {
    numero: u32,
    celdas: HashMap<String, &'a Celda>,
}

impl Fila<'_> {
    fn texto(&self, k: &str) -> String {
        self.celdas
            .get(&normalizar(k))
            .map(|c| c.texto.trim().to_string())
            .unwrap_or_default()
    }

    fn vacia(&self) -> bool {
        self.celdas.values().all(|c| c.vacia())
    }

    /// Monto: valor numérico exacto de la celda o texto escrito por la persona.
    fn monto(&self, k: &str, obligatorio: bool) -> Result<Option<Decimal>, String> {
        let Some(c) = self.celdas.get(&normalizar(k)).filter(|c| !c.vacia()) else {
            return if obligatorio {
                Err(format!("falta {k}"))
            } else {
                Ok(None)
            };
        };
        let d = match c.valor.as_deref() {
            Some(v) => Decimal::from_str(v)
                .or_else(|_| Decimal::from_scientific(v))
                .map(redondear)
                .map_err(|_| format!("{k} no es un número"))?,
            None => leer_monto(&c.texto).map_err(|e| e.0)?,
        };
        if d.is_sign_negative() && !d.is_zero() {
            return Err(format!("{k} no puede ser negativo"));
        }
        Ok(Some(d))
    }

    fn entero(&self, k: &str) -> Option<i64> {
        let c = self.celdas.get(&normalizar(k))?;
        c.valor
            .as_deref()
            .and_then(|v| Decimal::from_str(v).ok())
            .and_then(|d| i64::try_from(d.trunc()).ok())
            .or_else(|| c.texto.trim().parse().ok())
    }

    fn periodo(&self) -> Result<(i32, u32), String> {
        let anio = self.entero("Año").ok_or("falta el Año")?;
        if !(2000..=2100).contains(&anio) {
            return Err(format!("el año {anio} no es válido"));
        }
        let mes = self
            .entero("Mes")
            .or_else(|| {
                let t = normalizar(&self.texto("Mes"));
                NOMBRES_MES
                    .iter()
                    .position(|m| normalizar(m) == t)
                    .map(|i| i as i64 + 1)
            })
            .ok_or("falta el Mes o no es válido")?;
        if !(1..=12).contains(&mes) {
            return Err(format!("el mes {mes} no es válido"));
        }
        Ok((anio as i32, mes as u32))
    }
}

fn filas<'a>(hojas: &'a [Hoja], nombre: &str) -> Vec<Fila<'a>> {
    let Some(h) = hojas.iter().find(|h| h.nombre == nombre) else {
        return vec![];
    };
    let encabezados: HashMap<u32, String> = h
        .celdas
        .range((0, 0)..(1, 0))
        .map(|(&(_, c), celda)| (c, normalizar(celda.texto.trim())))
        .collect();
    let ultima = h.celdas.keys().map(|&(f, _)| f).max().unwrap_or(0);
    (1..=ultima)
        .map(|f| Fila {
            numero: f + 1,
            celdas: h
                .celdas
                .range((f, 0)..(f + 1, 0))
                .filter_map(|(&(_, c), celda)| Some((encabezados.get(&c)?.clone(), celda)))
                .collect(),
        })
        .filter(|f| !f.vacia())
        .collect()
}

/// Origen estable de una fila según su contenido; `n` distingue filas idénticas.
fn origen(hoja: &str, partes: &[String], n: usize) -> String {
    let h = Sha256::digest(format!("{hoja}|{}|{n}", partes.join("|")).as_bytes());
    let corto: String = h.iter().take(8).map(|b| format!("{b:02x}")).collect();
    format!("plantilla:{hoja}:{corto}")
}

struct Contador(HashMap<String, usize>);

impl Contador {
    fn siguiente(&mut self, clave: String) -> usize {
        let n = self.0.entry(clave).or_insert(0);
        *n += 1;
        *n
    }
}

/// Busca por nombre (sin mayúsculas ni tildes) o crea con `crear`.
fn por_nombre<T>(
    lista: &mut Vec<(String, i64)>,
    nombre: &str,
    crear: impl FnOnce() -> Result<T, Error>,
    id: impl Fn(&T) -> Option<i64>,
) -> Result<(i64, bool), Error> {
    let clave = normalizar(nombre.trim());
    if let Some((_, i)) = lista.iter().find(|(n, _)| *n == clave) {
        return Ok((*i, false));
    }
    let nuevo = crear()?;
    let i = id(&nuevo).ok_or_else(|| Error::Formato("registro sin id".into()))?;
    lista.push((clave, i));
    Ok((i, true))
}

fn mes_id(c: &Connection, (anio, mes): (i32, u32)) -> Result<i64, Error> {
    if let Some(id) = repo::mes_por_periodo(c, anio, mes)?.and_then(|m| m.id) {
        return Ok(id);
    }
    repo::guardar_mes(
        c,
        &Mes {
            id: None,
            anio,
            mes,
            cerrado: false,
        },
    )?
    .id
    .ok_or_else(|| Error::Formato("mes sin id".into()))
}

/// Importa una plantilla dentro de la transacción del importador. Cualquier fila con error
/// queda como diferencia: la importación falla y no se guarda nada.
pub fn importar(c: &Connection, hojas: &[Hoja], inf: &mut Informe) -> Result<(), Error> {
    let mut r = Resultado::default();
    let mut conteos = Conteos::default();
    let mut log = Vec::new();
    let rechazo =
        |hoja: &str, numero: u32, motivo: String, r: &mut Resultado, log: &mut Vec<LineaLog>| {
            log.push(LineaLog {
                origen: format!("{hoja}!fila {numero}"),
                accion: Accion::Rechazada,
                detalle: motivo.clone(),
            });
            r.diferencias.push(Diferencia {
                concepto: format!("{hoja}, fila {numero}: {motivo}"),
                hoja: None,
                app: None,
            });
        };

    let nombres = |v: Vec<(Option<i64>, String)>| -> Vec<(String, i64)> {
        v.into_iter()
            .filter_map(|(id, n)| Some((normalizar(&n), id?)))
            .collect()
    };
    let mut deudas = nombres(
        repo::listar_deudas(c)?
            .into_iter()
            .map(|d| (d.id, d.nombre))
            .collect(),
    );
    let mut tarjetas = nombres(
        repo::listar_tarjetas(c)?
            .into_iter()
            .map(|t| (t.id, t.nombre))
            .collect(),
    );
    let mut fuentes = nombres(
        repo::listar_items(c, Catalogo::Fuentes)?
            .into_iter()
            .map(|i| (i.id, i.nombre))
            .collect(),
    );
    let estados: Vec<(String, i64)> = nombres(
        repo::listar_estados(c)?
            .into_iter()
            .map(|e| (e.id, e.nombre))
            .collect(),
    );

    // Deudas.
    for f in filas(hojas, "Deudas") {
        let nombre = f.texto("Nombre");
        if nombre.is_empty() {
            rechazo(
                "Deudas",
                f.numero,
                "falta el Nombre".into(),
                &mut r,
                &mut log,
            );
            continue;
        }
        let total = match f.monto("Total", false) {
            Ok(t) => t,
            Err(e) => {
                rechazo("Deudas", f.numero, e, &mut r, &mut log);
                continue;
            }
        };
        let (id, nueva) = por_nombre(
            &mut deudas,
            &nombre,
            || {
                Ok(repo::guardar_deuda(
                    c,
                    &Deuda {
                        id: None,
                        nombre: nombre.clone(),
                        total,
                        ajuste: Decimal::ZERO,
                        saldada: false,
                        notas: f.texto("Notas"),
                        origen: None,
                    },
                )?)
            },
            |d| d.id,
        )?;
        if !nueva && total.is_some() {
            let mut d = repo::deuda(c, id)?;
            d.total = total;
            repo::guardar_deuda(c, &d)?;
        }
        conteos.deudas += 1;
        log.push(LineaLog {
            origen: format!("Deudas!fila {}", f.numero),
            accion: Accion::Importada,
            detalle: format!(
                "deuda \"{nombre}\"{}",
                if nueva { " (nueva)" } else { " (actualizada)" }
            ),
        });
    }

    // Tarjetas.
    for f in filas(hojas, "Tarjetas") {
        let nombre = f.texto("Nombre");
        let datos = (|| -> Result<_, String> {
            if nombre.is_empty() {
                return Err("falta el Nombre".into());
            }
            let dia = |k: &str| -> Result<Option<u8>, String> {
                match f.entero(k) {
                    None if f.texto(k).is_empty() => Ok(None),
                    Some(d) if (1..=31).contains(&d) => Ok(Some(d as u8)),
                    _ => Err(format!("{k} debe estar entre 1 y 31")),
                }
            };
            Ok((
                f.monto("Cupo", false)?,
                f.monto("Saldo", false)?,
                dia("Día de corte")?,
                dia("Día de pago")?,
            ))
        })();
        let (cupo, saldo, corte, pago) = match datos {
            Ok(d) => d,
            Err(e) => {
                rechazo("Tarjetas", f.numero, e, &mut r, &mut log);
                continue;
            }
        };
        let t = Tarjeta {
            id: None,
            nombre: nombre.clone(),
            cupo,
            saldo,
            dia_corte: corte,
            dia_pago: pago,
            origen: None,
        };
        let (id, nueva) = por_nombre(
            &mut tarjetas,
            &nombre,
            || Ok(repo::guardar_tarjeta(c, &t)?),
            |t| t.id,
        )?;
        if !nueva {
            let mut x = repo::tarjeta(c, id)?;
            x.cupo = cupo.or(x.cupo);
            x.saldo = saldo.or(x.saldo);
            x.dia_corte = corte.or(x.dia_corte);
            x.dia_pago = pago.or(x.dia_pago);
            repo::guardar_tarjeta(c, &x)?;
        }
        conteos.tarjetas += 1;
        log.push(LineaLog {
            origen: format!("Tarjetas!fila {}", f.numero),
            accion: Accion::Importada,
            detalle: format!("tarjeta \"{nombre}\""),
        });
    }

    // Gastos.
    let pendiente = repo::id_estado(c, "Pendiente")?;
    let mut contador = Contador(HashMap::new());
    let mut suma_hoja = Decimal::ZERO;
    let mut origenes = Vec::new();
    for f in filas(hojas, "Gastos") {
        let datos = (|| -> Result<_, String> {
            let periodo = f.periodo()?;
            let concepto = f.texto("Concepto");
            if concepto.is_empty() {
                return Err("falta el Concepto".into());
            }
            let monto = f.monto("Monto", true)?.unwrap_or_default();
            let estado = match f.texto("Estado") {
                e if e.is_empty() => pendiente,
                e => estados
                    .iter()
                    .find(|(n, _)| *n == normalizar(&e))
                    .map(|(_, i)| *i)
                    .ok_or_else(|| format!("el estado \"{e}\" no existe (mira la hoja Listas)"))?,
            };
            Ok((periodo, concepto, monto, estado))
        })();
        let (periodo, concepto, monto, estado_id) = match datos {
            Ok(d) => d,
            Err(e) => {
                rechazo("Gastos", f.numero, e, &mut r, &mut log);
                continue;
            }
        };
        let opcional = |k: &str| Some(f.texto(k)).filter(|t| !t.is_empty());
        let categoria_id = opcional("Categoría")
            .map(|n| repo::id_item(c, Catalogo::Categorias, n.trim()))
            .transpose()?;
        let deuda_id = match opcional("Deuda") {
            Some(n) => Some(
                por_nombre(
                    &mut deudas,
                    &n,
                    || {
                        Ok(repo::guardar_deuda(
                            c,
                            &Deuda {
                                id: None,
                                nombre: n.clone(),
                                total: None,
                                ajuste: Decimal::ZERO,
                                saldada: false,
                                notas: String::new(),
                                origen: None,
                            },
                        )?)
                    },
                    |d| d.id,
                )?
                .0,
            ),
            None => None,
        };
        let tarjeta_id = match opcional("Tarjeta") {
            Some(n) => Some(
                por_nombre(
                    &mut tarjetas,
                    &n,
                    || {
                        Ok(repo::guardar_tarjeta(
                            c,
                            &Tarjeta {
                                id: None,
                                nombre: n.clone(),
                                cupo: None,
                                saldo: None,
                                dia_corte: None,
                                dia_pago: None,
                                origen: None,
                            },
                        )?)
                    },
                    |t| t.id,
                )?
                .0,
            ),
            None => None,
        };
        let partes = vec![
            periodo.0.to_string(),
            periodo.1.to_string(),
            normalizar(&concepto),
            monto.normalize().to_string(),
        ];
        let n = contador.siguiente(format!("g|{}", partes.join("|")));
        let o = origen("Gastos", &partes, n);
        repo::guardar_gasto(
            c,
            &Gasto {
                id: None,
                mes_id: mes_id(c, periodo)?,
                concepto: concepto.clone(),
                monto,
                estado_id,
                categoria_id,
                deuda_id,
                tarjeta_id,
                desglose: None,
                nota: f.texto("Nota"),
                fuera_de_bloque: false,
                orden: i64::from(f.numero),
                origen: Some(o.clone()),
            },
        )?;
        suma_hoja += monto;
        origenes.push(o);
        conteos.gastos += 1;
        log.push(LineaLog {
            origen: format!("Gastos!fila {}", f.numero),
            accion: Accion::Importada,
            detalle: format!("gasto \"{concepto}\" {monto}"),
        });
    }

    // Ingresos.
    let mut suma_ingresos = Decimal::ZERO;
    let mut origenes_ingresos = Vec::new();
    for f in filas(hojas, "Ingresos") {
        let datos = (|| -> Result<_, String> {
            Ok((f.periodo()?, f.monto("Monto", true)?.unwrap_or_default()))
        })();
        let (periodo, monto) = match datos {
            Ok(d) => d,
            Err(e) => {
                rechazo("Ingresos", f.numero, e, &mut r, &mut log);
                continue;
            }
        };
        let fuente = Some(f.texto("Fuente"))
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "Sueldo".into());
        let (fuente_id, _) = por_nombre(
            &mut fuentes,
            &fuente,
            || Ok(repo::id_item(c, Catalogo::Fuentes, fuente.trim())?),
            |i| Some(*i),
        )?;
        let partes = vec![
            periodo.0.to_string(),
            periodo.1.to_string(),
            normalizar(&fuente),
            monto.normalize().to_string(),
        ];
        let n = contador.siguiente(format!("i|{}", partes.join("|")));
        let o = origen("Ingresos", &partes, n);
        repo::guardar_ingreso(
            c,
            &Ingreso {
                id: None,
                mes_id: mes_id(c, periodo)?,
                fuente_id,
                monto,
                desglose: None,
                nota: f.texto("Nota"),
                origen: Some(o.clone()),
            },
        )?;
        suma_ingresos += monto;
        origenes_ingresos.push(o);
        conteos.ingresos += 1;
        log.push(LineaLog {
            origen: format!("Ingresos!fila {}", f.numero),
            accion: Accion::Importada,
            detalle: format!("ingreso \"{fuente}\" {monto}"),
        });
    }

    // Conciliación: lo guardado debe sumar exactamente lo leído.
    let guardado_g: Decimal = repo::todos_gastos(c)?
        .iter()
        .filter(|g| g.origen.as_ref().is_some_and(|o| origenes.contains(o)))
        .map(|g| g.monto)
        .sum();
    let guardado_i: Decimal = repo::todos_ingresos(c)?
        .iter()
        .filter(|i| {
            i.origen
                .as_ref()
                .is_some_and(|o| origenes_ingresos.contains(o))
        })
        .map(|i| i.monto)
        .sum();
    for (concepto, hoja, app) in [
        ("Suma de gastos de la plantilla", suma_hoja, guardado_g),
        (
            "Suma de ingresos de la plantilla",
            suma_ingresos,
            guardado_i,
        ),
    ] {
        r.comparados += 1;
        if hoja != app {
            r.diferencias.push(Diferencia {
                concepto: concepto.into(),
                hoja: Some(hoja),
                app: Some(app),
            });
        }
    }
    inf.log.extend(log);
    inf.conteos = conteos;
    inf.conciliacion = r;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::escritura::escribir_ods;
    use crate::importar::importar_bytes;
    use std::io::Cursor;

    fn fila(v: &[&str]) -> Vec<Dato> {
        v.iter()
            .map(|t| {
                if t.is_empty() {
                    Dato::Vacio
                } else {
                    Dato::Texto((*t).to_string())
                }
            })
            .collect()
    }

    fn plantilla_llena(gastos: Vec<Vec<Dato>>) -> Vec<u8> {
        let c = sueldapp_almacen::abrir_memoria().unwrap();
        let mut t = tablas_plantilla(&c).unwrap();
        t.iter_mut().find(|t| t.nombre == "Gastos").unwrap().filas = gastos;
        t.iter_mut().find(|t| t.nombre == "Ingresos").unwrap().filas = vec![vec![
            Dato::Numero(2026.into()),
            texto("Octubre"),
            Dato::Vacio,
            texto("3.000.000"),
            Dato::Vacio,
        ]];
        t.iter_mut().find(|t| t.nombre == "Tarjetas").unwrap().filas =
            vec![fila(&["Tarjeta Verde", "5.000.000", "", "15", ""])];
        let mut b = Cursor::new(Vec::new());
        escribir_ods(&t, &mut b).unwrap();
        b.into_inner()
    }

    #[test]
    fn importa_plantilla_y_no_duplica() {
        let bytes = plantilla_llena(vec![
            fila(&[
                "2026", "10", "Arriendo", "950.000", "Pagado", "Hogar", "", "", "",
            ]),
            fila(&[
                "2026",
                "Octubre",
                "Almuerzo",
                "18000,5",
                "",
                "",
                "",
                "Tarjeta Verde",
                "nota",
            ]),
            fila(&["2026", "Octubre", "Almuerzo", "18000,5", "", "", "", "", ""]),
            fila(&[
                "2026",
                "11",
                "Bici – Cuota 1",
                "150000",
                "Pendiente",
                "",
                "Bici",
                "",
                "",
            ]),
        ]);
        let mut c = sueldapp_almacen::abrir_memoria().unwrap();
        let inf = importar_bytes(&mut c, "p.ods", &bytes, false).unwrap();
        assert!(inf.ok, "{:?}", inf.conciliacion.diferencias);
        assert_eq!(inf.formato, "plantilla para llenar");
        assert_eq!(inf.conteos.gastos, 4);
        let gastos = repo::todos_gastos(&c).unwrap();
        assert_eq!(gastos.len(), 4);
        assert!(gastos
            .iter()
            .any(|g| g.monto == Decimal::from_str("18000.5").unwrap()));
        assert_eq!(repo::listar_deudas(&c).unwrap()[0].nombre, "Bici");
        assert_eq!(repo::listar_tarjetas(&c).unwrap()[0].dia_corte, Some(15));
        // Mismo contenido otra vez (forzado): nada nuevo.
        let otra = importar_bytes(&mut c, "p.ods", &bytes, true).unwrap();
        assert!(otra.ok);
        assert_eq!(repo::todos_gastos(&c).unwrap().len(), 4);
        assert_eq!(repo::todos_ingresos(&c).unwrap().len(), 1);
    }

    #[test]
    fn una_fila_mala_no_guarda_nada() {
        let bytes = plantilla_llena(vec![
            fila(&["2026", "10", "Arriendo", "950.000", "", "", "", "", ""]),
            fila(&["2026", "13", "Mal", "abc", "Raro", "", "", "", ""]),
        ]);
        let mut c = sueldapp_almacen::abrir_memoria().unwrap();
        let inf = importar_bytes(&mut c, "p.ods", &bytes, false).unwrap();
        assert!(!inf.ok);
        assert!(inf.conciliacion.diferencias[0].concepto.contains("fila 3"));
        assert!(repo::todos_gastos(&c).unwrap().is_empty());
    }
}
