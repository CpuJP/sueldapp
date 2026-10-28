//! Importador: lleva la hoja original (o una exportación propia) a la base, en una sola
//! transacción, con log por fila y conciliación. Si la conciliación falla no se escribe nada.

use crate::conciliar;
use crate::formula::{separar, sumandos};
use crate::hoja::{self, BloqueHoja, GastoHoja};
use crate::lectura::{leer_libro, ref_celda, Hoja};
use crate::{escritura, Error};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use sueldapp_almacen::repo::{self, Catalogo};
use sueldapp_dominio::texto::{normalizar, raiz_concepto};
use sueldapp_dominio::{Decimal, Deuda, Gasto, Ingreso, Mes, Tarjeta, NOMBRES_MES};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Accion {
    Importada,
    Transformada,
    Rechazada,
    Omitida,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineaLog {
    pub origen: String,
    pub accion: Accion,
    pub detalle: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Conteos {
    pub meses: usize,
    pub gastos: usize,
    pub ingresos: usize,
    pub deudas: usize,
    pub tarjetas: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Informe {
    pub archivo: String,
    pub huella: String,
    /// `true` si se guardó (conciliación sin diferencias no aceptadas).
    pub ok: bool,
    /// El mismo archivo ya se había importado: no se cambió nada.
    pub ya_importado: bool,
    /// `"hoja original"` o `"exportación de Sueldapp"`.
    pub formato: String,
    pub conteos: Conteos,
    pub conciliacion: conciliar::Resultado,
    pub log: Vec<LineaLog>,
}

impl Informe {
    pub fn log_texto(&self) -> String {
        self.log
            .iter()
            .map(|l| {
                format!(
                    "{:<22} {:<12} {}",
                    l.origen,
                    format!("{:?}", l.accion),
                    l.detalle
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn resumen(&self) -> String {
        let c = &self.conteos;
        format!(
            "{}: {} meses, {} gastos, {} ingresos, {} deudas, {} tarjetas. {} valores comparados, {} diferencias, {} aceptadas.",
            if self.ok { "Importación correcta" } else { "Importación fallida" },
            c.meses,
            c.gastos,
            c.ingresos,
            c.deudas,
            c.tarjetas,
            self.conciliacion.comparados,
            self.conciliacion.diferencias.len(),
            self.conciliacion.aceptadas.len()
        )
    }
}

pub fn huella(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Importa un archivo. Con `forzar = false`, un archivo ya importado con éxito no se vuelve a
/// procesar. El archivo de origen solo se lee.
pub fn importar_archivo(c: &mut Connection, ruta: &Path, forzar: bool) -> Result<Informe, Error> {
    let bytes = std::fs::read(ruta)?;
    let nombre = ruta
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    importar_bytes(c, &nombre, &bytes, forzar)
}

/// Importa con las reglas genéricas más las personales (`Reglas::cargar`).
pub fn importar_bytes(
    c: &mut Connection,
    archivo: &str,
    bytes: &[u8],
    forzar: bool,
) -> Result<Informe, Error> {
    importar_bytes_con(c, archivo, bytes, forzar, &Reglas::cargar()?)
}

pub fn importar_bytes_con(
    c: &mut Connection,
    archivo: &str,
    bytes: &[u8],
    forzar: bool,
    reglas: &Reglas,
) -> Result<Informe, Error> {
    let huella = huella(bytes);
    let previa: Option<i64> = c
        .query_row(
            "SELECT id FROM importaciones WHERE huella = ?1 AND ok = 1",
            [&huella],
            |f| f.get(0),
        )
        .optional()?;
    let hojas = leer_libro(bytes)?;
    let propio = escritura::es_formato_propio(&hojas);
    let plantilla = !propio && crate::plantilla::es_plantilla(&hojas);
    let mut informe = Informe {
        archivo: archivo.to_string(),
        huella,
        ok: false,
        ya_importado: false,
        formato: if propio {
            "exportación de Sueldapp".into()
        } else if plantilla {
            "plantilla para llenar".into()
        } else {
            "hoja original".into()
        },
        conteos: Conteos::default(),
        conciliacion: conciliar::Resultado::default(),
        log: Vec::new(),
    };
    if previa.is_some() && !forzar {
        informe.ok = true;
        informe.ya_importado = true;
        informe.log.push(LineaLog {
            origen: archivo.into(),
            accion: Accion::Omitida,
            detalle: "este archivo ya se importó; no se cambió nada".into(),
        });
        return Ok(informe);
    }

    let tx = c.transaction()?;
    let r = if propio {
        escritura::restaurar(&tx, &hojas, &mut informe)
    } else if plantilla {
        crate::plantilla::importar(&tx, &hojas, &mut informe)
    } else {
        importar_original(&tx, &hojas, &mut informe, reglas)
    };
    match r {
        Ok(()) if informe.conciliacion.diferencias.is_empty() => {
            tx.commit()?;
            informe.ok = true;
        }
        Ok(()) => drop(tx),
        Err(e) => {
            drop(tx);
            return Err(e);
        }
    }
    let log = serde_json::to_string(&informe.log).unwrap_or_default();
    repo::registrar_importacion(
        c,
        archivo,
        &informe.huella,
        informe.ok,
        &informe.resumen(),
        &log,
    )?;
    Ok(informe)
}

/// Reglas de clasificación por concepto. Las genéricas vienen en el código; las personales
/// (nombres propios, tarjetas, alias de deudas) se leen de `reglas.txt`, que no se versiona.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reglas {
    /// (palabras de la raíz del concepto, categoría). Gana la primera que coincide.
    pub categorias: Vec<(String, String)>,
    /// (palabra del concepto, nombre de la tarjeta de crédito).
    pub tarjetas: Vec<(String, String)>,
    /// (nombre normalizado de una deuda sin fórmula de pago, raíz del concepto de sus cuotas).
    pub alias_deuda: Vec<(String, String)>,
}

/// Categorías genéricas, válidas para cualquier persona.
const CATEGORIAS_GENERICAS: &[(&str, &str)] = &[
    ("avance", "Préstamos"),
    ("prestamo", "Préstamos"),
    ("credito", "Préstamos"),
    ("tigo", "Servicios"),
    ("claro", "Servicios"),
    ("movistar", "Servicios"),
    ("internet", "Servicios"),
    ("netflix", "Servicios"),
    ("spotify", "Servicios"),
    ("google", "Servicios"),
    ("semestre", "Educación"),
    ("matricula", "Educación"),
    ("curso", "Educación"),
    ("arriendo", "Hogar"),
    ("casa", "Hogar"),
    ("gas", "Hogar"),
    ("agua", "Hogar"),
    ("luz", "Hogar"),
    ("energia", "Hogar"),
    ("mercado", "Alimentación"),
    ("almuerzo", "Alimentación"),
    ("restaurante", "Alimentación"),
    ("gimnasio", "Salud y bienestar"),
    ("medico", "Salud y bienestar"),
    ("ropa", "Ropa y calzado"),
    ("zapatos", "Ropa y calzado"),
];

impl Reglas {
    pub fn genericas() -> Reglas {
        Reglas {
            categorias: CATEGORIAS_GENERICAS
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
            ..Default::default()
        }
    }

    /// Lee reglas en formato `tipo: clave = valor` (tipos: `categoria`, `tarjeta`, `alias`;
    /// `#` comenta). Las reglas del texto van antes que las genéricas.
    pub fn desde_texto(texto: &str) -> Result<Reglas, Error> {
        let mut propias = Reglas::default();
        for (n, linea) in texto.lines().enumerate() {
            let l = linea.trim();
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            let error = || {
                Error::Formato(format!(
                    "reglas.txt, línea {}: se esperaba \"tipo: clave = valor\"",
                    n + 1
                ))
            };
            let (tipo, resto) = l.split_once(':').ok_or_else(error)?;
            let (clave, valor) = resto.split_once('=').ok_or_else(error)?;
            let (clave, valor) = (clave.trim(), valor.trim().to_string());
            if clave.is_empty() || valor.is_empty() {
                return Err(error());
            }
            match normalizar(tipo.trim()).as_str() {
                "categoria" => propias.categorias.push((raiz_concepto(clave), valor)),
                "tarjeta" => propias.tarjetas.push((raiz_concepto(clave), valor)),
                // La clave es el nombre de la deuda; el valor, la raíz del concepto de sus cuotas.
                "alias" => propias
                    .alias_deuda
                    .push((normalizar(clave), raiz_concepto(&valor))),
                _ => return Err(error()),
            }
        }
        propias.categorias.extend(Reglas::genericas().categorias);
        Ok(propias)
    }

    /// `SUELDAPP_REGLAS`, o `reglas.txt` junto a la base de datos.
    pub fn ruta() -> std::path::PathBuf {
        std::env::var_os("SUELDAPP_REGLAS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| sueldapp_almacen::ruta_por_defecto().with_file_name("reglas.txt"))
    }

    /// Reglas genéricas más las del archivo personal, si existe.
    pub fn cargar() -> Result<Reglas, Error> {
        match std::fs::read_to_string(Reglas::ruta()) {
            Ok(t) => Reglas::desde_texto(&t),
            Err(_) => Ok(Reglas::genericas()),
        }
    }

    fn coincide(raiz: &str, clave: &str) -> bool {
        format!(" {raiz} ").contains(&format!(" {clave} "))
    }

    /// Categoría sugerida para un concepto, o `None`.
    pub fn categoria(&self, concepto: &str) -> Option<&str> {
        let raiz = raiz_concepto(concepto);
        self.categorias
            .iter()
            .find(|(clave, _)| Reglas::coincide(&raiz, clave))
            .map(|(_, cat)| cat.as_str())
    }

    /// Tarjeta de crédito a la que pertenece un pago, o `None`.
    pub fn tarjeta(&self, concepto: &str) -> Option<&str> {
        let raiz = raiz_concepto(concepto);
        self.tarjetas
            .iter()
            .find(|(clave, _)| Reglas::coincide(&raiz, clave))
            .map(|(_, t)| t.as_str())
    }
}

/// Asigna categoría a los gastos que no tienen, según las reglas. No toca los que ya tienen
/// categoría. Devuelve cuántos cambió.
pub fn categorizar_sin_categoria(c: &mut Connection, reglas: &Reglas) -> Result<usize, Error> {
    let tx = c.transaction()?;
    let mut n = 0;
    for mut g in repo::todos_gastos(&tx)?
        .into_iter()
        .filter(|g| g.categoria_id.is_none())
    {
        let categoria = if g.tarjeta_id.is_some() {
            Some("Tarjetas")
        } else {
            reglas.categoria(&g.concepto)
        };
        if let Some(nombre) = categoria {
            g.categoria_id = Some(repo::id_item(&tx, Catalogo::Categorias, nombre)?);
            repo::guardar_gasto(&tx, &g)?;
            n += 1;
        }
    }
    tx.commit()?;
    Ok(n)
}

struct Ids {
    pagado: i64,
    pendiente: i64,
    no: i64,
    aplazado: i64,
    sin_estado: i64,
    tarjetas: i64,
    sueldo: i64,
    prima: i64,
}

impl Ids {
    fn cargar(c: &Connection) -> Result<Ids, Error> {
        Ok(Ids {
            pagado: repo::id_estado(c, "Pagado")?,
            pendiente: repo::id_estado(c, "Pendiente")?,
            no: repo::id_estado(c, "No")?,
            aplazado: repo::id_estado(c, "Aplazado")?,
            sin_estado: repo::id_estado(c, "Sin estado")?,
            tarjetas: repo::id_item(c, Catalogo::Categorias, "Tarjetas")?,
            sueldo: repo::id_item(c, Catalogo::Fuentes, "Sueldo")?,
            prima: repo::id_item(c, Catalogo::Fuentes, "Prima")?,
        })
    }

    /// Estado de la app para el texto de la hoja, y nota si hubo transformación.
    fn estado(&self, texto: &str) -> (i64, bool, Option<String>) {
        let (id, nombre) = match texto.to_uppercase().as_str() {
            "OK" => (self.pagado, "Pagado"),
            "PENDIENTE" => (self.pendiente, "Pendiente"),
            "NO" => (self.no, "No"),
            "APLAZADO" => (self.aplazado, "Aplazado"),
            "" => return (self.sin_estado, false, None),
            _ => {
                return (
                    self.sin_estado,
                    false,
                    Some(format!("estado desconocido \"{texto}\" → Sin estado")),
                )
            }
        };
        let canonico = texto == texto.to_uppercase();
        let nota = (!canonico).then(|| format!("estado \"{texto}\" → {nombre}"));
        (id, id == self.pagado, nota)
    }
}

/// Desglose: la fórmula, salvo que sea solo un número (`=1000`).
fn desglose(formula: &Option<String>) -> Option<String> {
    let f = formula.as_ref()?;
    let cuerpo = f.trim_start_matches('=').trim_start_matches('+');
    (cuerpo.parse::<Decimal>().is_err()).then(|| f.clone())
}

fn asegurar_tarjeta(c: &Connection, nombre: &str, origen: &str) -> Result<i64, Error> {
    if let Some(t) = repo::listar_tarjetas(c)?
        .into_iter()
        .find(|t| t.origen.as_deref() == Some(origen))
    {
        // No se pisa lo que el usuario haya editado (cupo, saldo, días).
        return t.id.ok_or(Error::Formato("tarjeta sin id".into()));
    }
    let t = repo::guardar_tarjeta(
        c,
        &Tarjeta {
            id: None,
            nombre: nombre.into(),
            cupo: None,
            saldo: None,
            dia_corte: None,
            dia_pago: None,
            origen: Some(origen.into()),
        },
    )?;
    t.id.ok_or(Error::Formato("tarjeta sin id".into()))
}

fn importar_original(
    c: &Connection,
    hojas: &[Hoja],
    inf: &mut Informe,
    reglas: &Reglas,
) -> Result<(), Error> {
    let nomina = hojas
        .iter()
        .find(|h| normalizar(&h.nombre) == "nomina")
        .ok_or_else(|| {
            Error::Formato(
                "No encontré la pestaña \"Nómina\". El archivo no tiene la estructura de la hoja original."
                    .into(),
            )
        })?;
    for h in hojas.iter().filter(|h| !std::ptr::eq(*h, nomina)) {
        omitir_hoja(h, inf);
    }
    let n = hoja::extraer(nomina)?;
    let o = |f: u32, col: u32| format!("{}!{}", nomina.nombre, ref_celda(f, col));
    let prefijo = format!("{}!", nomina.nombre);
    let log = &mut inf.log;
    let mut linea = |origen: String, accion: Accion, detalle: String| {
        log.push(LineaLog {
            origen,
            accion,
            detalle,
        })
    };
    for a in &n.avisos {
        let accion = if a.rechazada {
            Accion::Rechazada
        } else {
            Accion::Transformada
        };
        linea(format!("{prefijo}{}", a.celda), accion, a.detalle.clone());
    }
    for a in &n.omitidas {
        linea(
            format!("{prefijo}{}", a.celda),
            Accion::Omitida,
            a.detalle.clone(),
        );
    }

    let ids = Ids::cargar(c)?;
    let mut tarjetas_usadas: HashMap<String, i64> = HashMap::new();

    // Índice de gastos de la hoja por celda del monto.
    let por_celda: HashMap<(u32, u32), &GastoHoja> = n
        .bloques
        .iter()
        .flat_map(|b| &b.gastos)
        .map(|g| ((g.monto.fila, g.monto.col), g))
        .collect();

    // Deudas.
    let mut deuda_ids = Vec::new();
    let mut deuda_sin_formula = Vec::new();
    let mut vinculo: HashMap<(u32, u32), usize> = HashMap::new();
    let mut raices: Vec<BTreeSet<String>> = Vec::new();
    for (i, d) in n.deudas.iter().enumerate() {
        let terminos = d
            .pago
            .as_ref()
            .and_then(|p| p.formula.as_deref())
            .and_then(sumandos);
        let sin_formula = terminos.is_none();
        let (ajuste, refs) = terminos.map(|t| separar(&t)).unwrap_or_default();
        let mut r = BTreeSet::new();
        for celda in &refs {
            match por_celda.get(celda) {
                Some(g) => {
                    vinculo.insert(*celda, i);
                    r.insert(raiz_concepto(&g.concepto));
                }
                None => linea(
                    o(d.fila, 1),
                    Accion::Rechazada,
                    format!(
                        "la fórmula de pago apunta a {} que no es un gasto",
                        ref_celda(celda.0, celda.1)
                    ),
                ),
            }
        }
        let clave = normalizar(&d.nombre);
        for (nombre, alias) in &reglas.alias_deuda {
            if clave == *nombre {
                r.insert(alias.clone());
            }
        }
        raices.push(r);
        let notas = d
            .total
            .as_ref()
            .and_then(|t| desglose(&t.formula))
            .map(|f| format!("Total en la hoja: {f}"))
            .unwrap_or_default();
        let guardada = repo::guardar_deuda(
            c,
            &Deuda {
                id: None,
                nombre: d.nombre.clone(),
                total: d.total.as_ref().map(|t| t.valor),
                ajuste,
                saldada: d.saldada,
                notas,
                origen: Some(o(d.fila, 0)),
            },
        )?;
        let detalle = if sin_formula {
            format!(
                "deuda \"{}\": el pago fijo de la hoja no tiene fórmula; se ignora y el total sale de sus cuotas (decisión del usuario)",
                d.nombre
            )
        } else {
            format!(
                "deuda \"{}\": {} cuotas vinculadas por fórmula, ajuste {}",
                d.nombre,
                refs.len(),
                ajuste
            )
        };
        linea(
            o(d.fila, 0),
            if sin_formula {
                Accion::Transformada
            } else {
                Accion::Importada
            },
            detalle,
        );
        deuda_ids.push(guardada.id.ok_or(Error::Formato("deuda sin id".into()))?);
        deuda_sin_formula.push(sin_formula);
    }

    // Meses, ingresos y gastos.
    let mut mes_ids = Vec::new();
    let mut suma_sin_formula: HashMap<usize, Decimal> = HashMap::new();
    let mut conteos = Conteos {
        deudas: n.deudas.len(),
        ..Default::default()
    };
    for b in &n.bloques {
        let col = b
            .sueldo
            .as_ref()
            .map(|v| v.col - 1)
            .or(b.gastos.first().map(|g| g.monto.col - 1))
            .unwrap_or(0);
        let mes = repo::guardar_mes(
            c,
            &Mes {
                id: None,
                anio: b.anio,
                mes: b.mes,
                cerrado: b.cerrado,
            },
        )?;
        let mes_id = mes.id.ok_or(Error::Formato("mes sin id".into()))?;
        mes_ids.push(mes_id);
        conteos.meses += 1;
        linea(
            o(b.fila, col),
            Accion::Importada,
            format!(
                "mes {} {}{}",
                NOMBRES_MES[b.mes as usize - 1],
                b.anio,
                if b.cerrado { " (cerrado)" } else { "" }
            ),
        );
        conteos.ingresos += importar_ingresos(c, b, mes_id, &ids, &o, &mut linea)?;

        for g in &b.gastos {
            let celda = (g.monto.fila, g.monto.col);
            let (estado_id, _, nota_estado) = ids.estado(&g.estado);
            let mut notas: Vec<String> = nota_estado.into_iter().collect();
            let deuda = match vinculo.get(&celda) {
                Some(&i) => Some(i),
                None if estado_id == ids.pendiente || estado_id == ids.sin_estado => {
                    let raiz = raiz_concepto(&g.concepto);
                    let candidatas: Vec<usize> = raices
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.contains(&raiz))
                        .map(|(i, _)| i)
                        .collect();
                    match candidatas.as_slice() {
                        [i] => {
                            notas.push(format!(
                                "vinculado por nombre a la deuda \"{}\"",
                                n.deudas[*i].nombre
                            ));
                            Some(*i)
                        }
                        _ => None,
                    }
                }
                None => None,
            };
            if let Some(i) = deuda.filter(|i| deuda_sin_formula[*i]) {
                *suma_sin_formula.entry(i).or_default() += g.monto.valor;
            }
            let tarjeta = match reglas.tarjeta(&g.concepto) {
                Some(nombre) => {
                    let origen = format!("tarjeta:{}", normalizar(nombre));
                    let id = match tarjetas_usadas.get(&origen) {
                        Some(id) => *id,
                        None => asegurar_tarjeta(c, nombre, &origen)?,
                    };
                    tarjetas_usadas.insert(origen, id);
                    Some(id)
                }
                None => None,
            };
            let categoria = match tarjeta {
                Some(_) => Some(ids.tarjetas),
                None => reglas
                    .categoria(&g.concepto)
                    .map(|n| repo::id_item(c, Catalogo::Categorias, n))
                    .transpose()?,
            };
            if g.fuera_de_bloque {
                notas.push(format!(
                    "fuera de cualquier mes en la hoja: asignado a {} {} sin contar en sus totales",
                    NOMBRES_MES[b.mes as usize - 1],
                    b.anio
                ));
            }
            repo::guardar_gasto(
                c,
                &Gasto {
                    id: None,
                    mes_id,
                    concepto: g.concepto.clone(),
                    monto: g.monto.valor,
                    estado_id,
                    categoria_id: categoria,
                    deuda_id: deuda.map(|i| deuda_ids[i]),
                    tarjeta_id: tarjeta,
                    desglose: desglose(&g.monto.formula),
                    nota: String::new(),
                    fuera_de_bloque: g.fuera_de_bloque,
                    orden: i64::from(g.monto.fila),
                    origen: Some(o(celda.0, celda.1)),
                },
            )?;
            conteos.gastos += 1;
            let accion = if notas.is_empty() {
                Accion::Importada
            } else {
                Accion::Transformada
            };
            let mut detalle = format!("gasto \"{}\" {}", g.concepto, g.monto.valor);
            if !notas.is_empty() {
                detalle = format!("{detalle}: {}", notas.join("; "));
            }
            linea(o(celda.0, celda.1), accion, detalle);
        }
    }

    // Deudas sin fórmula: total = suma de sus cuotas.
    for (i, total) in suma_sin_formula {
        let mut d = repo::deuda(c, deuda_ids[i])?;
        d.total = Some(total);
        d.ajuste = Decimal::ZERO;
        repo::guardar_deuda(c, &d)?;
    }

    conteos.tarjetas = tarjetas_usadas.len();
    inf.conteos = conteos;
    inf.conciliacion =
        conciliar::conciliar(c, &n, &prefijo, &mes_ids, &deuda_ids, &deuda_sin_formula)?;
    Ok(())
}

fn importar_ingresos(
    c: &Connection,
    b: &BloqueHoja,
    mes_id: i64,
    ids: &Ids,
    o: &impl Fn(u32, u32) -> String,
    linea: &mut impl FnMut(String, Accion, String),
) -> Result<usize, Error> {
    let mut n = 0;
    let prima_celda = b.prima.as_ref().map(|p| (p.fila, p.col));
    let mut prima_en_sueldo = false;
    if let Some(s) = &b.sueldo {
        let refs = s
            .formula
            .as_deref()
            .and_then(sumandos)
            .map(|t| separar(&t).1)
            .unwrap_or_default();
        prima_en_sueldo = prima_celda.is_some_and(|p| refs.contains(&p));
        let monto = match (&b.prima, prima_en_sueldo) {
            (Some(p), true) => s.valor - p.valor,
            _ => s.valor,
        };
        repo::guardar_ingreso(
            c,
            &Ingreso {
                id: None,
                mes_id,
                fuente_id: ids.sueldo,
                monto,
                desglose: desglose(&s.formula),
                nota: String::new(),
                origen: Some(o(s.fila, s.col)),
            },
        )?;
        n += 1;
        let detalle = if prima_en_sueldo {
            format!("sueldo {monto} (la hoja suma la prima en esta celda; la prima va aparte)")
        } else {
            format!("sueldo {monto}")
        };
        let accion = if prima_en_sueldo {
            Accion::Transformada
        } else {
            Accion::Importada
        };
        linea(o(s.fila, s.col), accion, detalle);
    } else {
        linea(
            o(b.fila, 0),
            Accion::Rechazada,
            "el mes no tiene fila Sueldo".into(),
        );
    }
    if let Some(p) = &b.prima {
        repo::guardar_ingreso(
            c,
            &Ingreso {
                id: None,
                mes_id,
                fuente_id: ids.prima,
                monto: p.valor,
                desglose: desglose(&p.formula),
                nota: String::new(),
                origen: Some(o(p.fila, p.col)),
            },
        )?;
        n += 1;
        let detalle = if prima_en_sueldo {
            format!("prima {}", p.valor)
        } else {
            format!("prima {} (la hoja no la suma al sueldo)", p.valor)
        };
        linea(o(p.fila, p.col), Accion::Importada, detalle);
    }
    Ok(n)
}

fn omitir_hoja(h: &Hoja, inf: &mut Informe) {
    let mut filas: Vec<u32> = h.celdas.keys().map(|&(f, _)| f).collect();
    filas.dedup();
    for f in filas {
        let textos: Vec<&str> = h
            .celdas
            .range((f, 0)..(f + 1, 0))
            .map(|(_, c)| c.texto.trim())
            .filter(|t| !t.is_empty())
            .collect();
        let primera = h
            .celdas
            .range((f, 0)..(f + 1, 0))
            .next()
            .map(|(&(f, c), _)| ref_celda(f, c))
            .unwrap_or_default();
        inf.log.push(LineaLog {
            origen: format!("{}!{primera}", h.nombre),
            accion: Accion::Omitida,
            detalle: format!(
                "pestaña omitida por decisión del usuario (cotización abandonada): {}",
                textos.join(" | ")
            ),
        });
    }
}

/// Conteos actuales de la base (para el informe de "ya importado" y la interfaz).
pub fn conteos_base(c: &Connection) -> Result<Conteos, Error> {
    let n = |t: &str| -> Result<usize, Error> {
        let v: i64 = c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |f| f.get(0))?;
        Ok(v as usize)
    };
    Ok(Conteos {
        meses: n("meses")?,
        gastos: n("gastos")?,
        ingresos: n("ingresos")?,
        deudas: n("deudas")?,
        tarjetas: n("tarjetas")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desgloses() {
        assert_eq!(desglose(&Some("=1000".into())), None);
        assert_eq!(desglose(&Some("=+1000".into())), None);
        assert_eq!(desglose(&Some("=500*2".into())), Some("=500*2".into()));
        assert_eq!(desglose(&None), None);
    }

    #[test]
    fn reglas_genericas_y_personales() {
        let g = Reglas::genericas();
        assert_eq!(g.categoria("Arriendo"), Some("Hogar"));
        assert_eq!(g.categoria("Visita Gas"), Some("Hogar"));
        assert_eq!(
            g.categoria("Cuota 2 - Ropa - FINAL"),
            Some("Ropa y calzado")
        );
        assert_eq!(g.categoria("Algo nuevo"), None);
        let r = Reglas::desde_texto(
            "# comentario\ncategoria: cel ana = Tecnología\ncategoria: ana = Familia\n\
             tarjeta: banco azul = Azul\nalias: Moto Nueva = moto",
        )
        .unwrap();
        assert_eq!(r.categoria("Cel Ana - Cuota 2"), Some("Tecnología"));
        assert_eq!(r.categoria("Ana"), Some("Familia"));
        assert_eq!(r.categoria("Mercado"), Some("Alimentación"));
        assert_eq!(r.tarjeta("Cuota - Tarjeta Banco Azul"), Some("Azul"));
        assert_eq!(
            r.alias_deuda,
            vec![("moto nueva".to_string(), "moto".to_string())]
        );
        assert!(Reglas::desde_texto("sin formato").is_err());
    }

    #[test]
    fn estados() {
        let ids = Ids {
            pagado: 1,
            pendiente: 2,
            no: 3,
            aplazado: 4,
            sin_estado: 5,
            tarjetas: 0,
            sueldo: 0,
            prima: 0,
        };
        assert_eq!(ids.estado("OK"), (1, true, None));
        assert_eq!(ids.estado("Ok").0, 1);
        assert!(ids.estado("Ok").2.is_some());
        assert_eq!(ids.estado(""), (5, false, None));
        assert_eq!(ids.estado("raro").0, 5);
    }
}
