//! Exportación propia a `.ods` (una hoja por tabla, montos escritos desde `Decimal`) y
//! restauración de esa exportación.

use crate::conciliar::{Diferencia, Resultado};
use crate::importar::{Accion, Conteos, Informe, LineaLog};
use crate::lectura::Hoja;
use crate::Error;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::io::Write;
use std::str::FromStr;
use sueldapp_almacen::repo::{self, Catalogo};
use sueldapp_dominio::calculos::totales_mes;
use sueldapp_dominio::{Decimal, Deuda, Gasto, Ingreso, Tarjeta};

/// Hoja marcadora de una exportación de Sueldapp.
const MARCA: &str = "Sueldapp";

#[derive(Debug, Clone, PartialEq)]
pub enum Dato {
    Texto(String),
    Numero(Decimal),
    Vacio,
}

impl From<&str> for Dato {
    fn from(s: &str) -> Self {
        Dato::Texto(s.to_string())
    }
}
impl From<String> for Dato {
    fn from(s: String) -> Self {
        Dato::Texto(s)
    }
}
impl From<Decimal> for Dato {
    fn from(d: Decimal) -> Self {
        Dato::Numero(d)
    }
}
impl From<i64> for Dato {
    fn from(n: i64) -> Self {
        Dato::Numero(Decimal::from(n))
    }
}
impl From<bool> for Dato {
    fn from(b: bool) -> Self {
        Dato::Numero(Decimal::from(u8::from(b)))
    }
}
impl<T: Into<Dato>> From<Option<T>> for Dato {
    fn from(o: Option<T>) -> Self {
        o.map(Into::into).unwrap_or(Dato::Vacio)
    }
}

#[derive(Debug, Clone)]
pub struct Tabla {
    pub nombre: &'static str,
    pub encabezados: Vec<&'static str>,
    pub filas: Vec<Vec<Dato>>,
}

macro_rules! fila {
    ($($v:expr),* $(,)?) => { vec![$(Dato::from($v)),*] };
}

/// Todas las tablas para exportar. Las columnas tras `origen` en Gastos/Ingresos son solo de
/// lectura (nombres legibles) y se ignoran al restaurar.
pub fn tablas(c: &Connection) -> Result<Vec<Tabla>, Error> {
    let meses = repo::listar_meses(c)?;
    let estados = repo::listar_estados(c)?;
    let categorias = repo::listar_items(c, Catalogo::Categorias)?;
    let fuentes = repo::listar_items(c, Catalogo::Fuentes)?;
    let deudas = repo::listar_deudas(c)?;
    let tarjetas = repo::listar_tarjetas(c)?;
    let gastos = repo::todos_gastos(c)?;
    let ingresos = repo::todos_ingresos(c)?;

    let nombre_de = |v: &[(Option<i64>, String)], id: Option<i64>| -> Dato {
        id.and_then(|id| v.iter().find(|(i, _)| *i == Some(id)))
            .map(|(_, n)| Dato::Texto(n.clone()))
            .unwrap_or(Dato::Vacio)
    };
    let n_estados: Vec<_> = estados.iter().map(|e| (e.id, e.nombre.clone())).collect();
    let n_cat: Vec<_> = categorias
        .iter()
        .map(|e| (e.id, e.nombre.clone()))
        .collect();
    let n_fue: Vec<_> = fuentes.iter().map(|e| (e.id, e.nombre.clone())).collect();
    let n_deu: Vec<_> = deudas.iter().map(|e| (e.id, e.nombre.clone())).collect();
    let n_tar: Vec<_> = tarjetas.iter().map(|e| (e.id, e.nombre.clone())).collect();
    let periodo: HashMap<i64, (i32, u32)> = meses
        .iter()
        .filter_map(|m| Some((m.id?, (m.anio, m.mes))))
        .collect();
    let anio_mes = |id: i64| periodo.get(&id).copied().unwrap_or((0, 0));

    let mut resumen = Vec::new();
    for m in &meses {
        let Some(id) = m.id else { continue };
        let g: Vec<_> = gastos.iter().filter(|x| x.mes_id == id).cloned().collect();
        let i: Vec<_> = ingresos
            .iter()
            .filter(|x| x.mes_id == id)
            .cloned()
            .collect();
        let t = totales_mes(&g, &i, |e| {
            estados.iter().any(|x| x.id == Some(e) && x.cuenta_pagado)
        });
        resumen.push(fila![
            i64::from(m.anio),
            i64::from(m.mes),
            t.ingreso,
            t.total,
            t.pagado,
            t.restante,
            t.restante_pagado
        ]);
    }

    Ok(vec![
        Tabla {
            nombre: MARCA,
            encabezados: vec!["clave", "valor"],
            filas: vec![
                fila!["formato", "exportación de Sueldapp"],
                fila!["version", 1i64],
                fila!["moneda", "COP"],
            ],
        },
        Tabla {
            nombre: "Meses",
            encabezados: vec!["id", "anio", "mes", "cerrado"],
            filas: meses
                .iter()
                .map(|m| fila![m.id, i64::from(m.anio), i64::from(m.mes), m.cerrado])
                .collect(),
        },
        Tabla {
            nombre: "Estados",
            encabezados: vec!["id", "nombre", "cuenta_pagado", "orden"],
            filas: estados
                .iter()
                .map(|e| fila![e.id, e.nombre.clone(), e.cuenta_pagado, e.orden])
                .collect(),
        },
        Tabla {
            nombre: "Categorias",
            encabezados: vec!["id", "nombre"],
            filas: categorias
                .iter()
                .map(|e| fila![e.id, e.nombre.clone()])
                .collect(),
        },
        Tabla {
            nombre: "Fuentes",
            encabezados: vec!["id", "nombre"],
            filas: fuentes
                .iter()
                .map(|e| fila![e.id, e.nombre.clone()])
                .collect(),
        },
        Tabla {
            nombre: "Deudas",
            encabezados: vec![
                "id", "nombre", "total", "ajuste", "saldada", "notas", "origen",
            ],
            filas: deudas
                .iter()
                .map(|d| {
                    fila![
                        d.id,
                        d.nombre.clone(),
                        d.total,
                        d.ajuste,
                        d.saldada,
                        d.notas.clone(),
                        d.origen.clone()
                    ]
                })
                .collect(),
        },
        Tabla {
            nombre: "Tarjetas",
            encabezados: vec![
                "id",
                "nombre",
                "cupo",
                "saldo",
                "dia_corte",
                "dia_pago",
                "origen",
            ],
            filas: tarjetas
                .iter()
                .map(|t| {
                    fila![
                        t.id,
                        t.nombre.clone(),
                        t.cupo,
                        t.saldo,
                        t.dia_corte.map(i64::from),
                        t.dia_pago.map(i64::from),
                        t.origen.clone()
                    ]
                })
                .collect(),
        },
        Tabla {
            nombre: "Gastos",
            encabezados: vec![
                "id",
                "mes_id",
                "concepto",
                "monto",
                "estado_id",
                "categoria_id",
                "deuda_id",
                "tarjeta_id",
                "desglose",
                "nota",
                "fuera_de_bloque",
                "orden",
                "origen",
                "anio",
                "mes",
                "estado",
                "categoria",
                "deuda",
                "tarjeta",
            ],
            filas: gastos
                .iter()
                .map(|g| {
                    let (a, m) = anio_mes(g.mes_id);
                    let mut f = fila![
                        g.id,
                        g.mes_id,
                        g.concepto.clone(),
                        g.monto,
                        g.estado_id,
                        g.categoria_id,
                        g.deuda_id,
                        g.tarjeta_id,
                        g.desglose.clone(),
                        g.nota.clone(),
                        g.fuera_de_bloque,
                        g.orden,
                        g.origen.clone(),
                        i64::from(a),
                        i64::from(m)
                    ];
                    f.push(nombre_de(&n_estados, Some(g.estado_id)));
                    f.push(nombre_de(&n_cat, g.categoria_id));
                    f.push(nombre_de(&n_deu, g.deuda_id));
                    f.push(nombre_de(&n_tar, g.tarjeta_id));
                    f
                })
                .collect(),
        },
        Tabla {
            nombre: "Ingresos",
            encabezados: vec![
                "id",
                "mes_id",
                "fuente_id",
                "monto",
                "desglose",
                "nota",
                "origen",
                "anio",
                "mes",
                "fuente",
            ],
            filas: ingresos
                .iter()
                .map(|i| {
                    let (a, m) = anio_mes(i.mes_id);
                    let mut f = fila![
                        i.id,
                        i.mes_id,
                        i.fuente_id,
                        i.monto,
                        i.desglose.clone(),
                        i.nota.clone(),
                        i.origen.clone(),
                        i64::from(a),
                        i64::from(m)
                    ];
                    f.push(nombre_de(&n_fue, Some(i.fuente_id)));
                    f
                })
                .collect(),
        },
        Tabla {
            nombre: "Resumen",
            encabezados: vec![
                "anio",
                "mes",
                "ingreso",
                "total",
                "pagado",
                "restante",
                "restante_pagado",
            ],
            filas: resumen,
        },
    ])
}

fn escapar(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn celda_xml(d: &Dato) -> String {
    match d {
        Dato::Texto(t) => {
            let parrafos: String = t
                .split('\n')
                .map(|p| format!("<text:p>{}</text:p>", escapar(p)))
                .collect();
            format!(r#"<table:table-cell office:value-type="string">{parrafos}</table:table-cell>"#)
        }
        Dato::Numero(n) => {
            let v = n.normalize().to_string();
            format!(
                r#"<table:table-cell office:value-type="float" office:value="{v}"><text:p>{v}</text:p></table:table-cell>"#
            )
        }
        Dato::Vacio => "<table:table-cell/>".into(),
    }
}

/// Escribe las tablas como `.ods`.
pub fn escribir_ods(tablas: &[Tabla], destino: impl Write + std::io::Seek) -> Result<(), Error> {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3"><office:body><office:spreadsheet>"#,
    );
    for t in tablas {
        xml.push_str(&format!(
            r#"<table:table table:name="{}">"#,
            escapar(t.nombre)
        ));
        xml.push_str("<table:table-row>");
        for e in &t.encabezados {
            xml.push_str(&celda_xml(&Dato::from(*e)));
        }
        xml.push_str("</table:table-row>");
        for f in &t.filas {
            xml.push_str("<table:table-row>");
            for d in f {
                xml.push_str(&celda_xml(d));
            }
            xml.push_str("</table:table-row>");
        }
        xml.push_str("</table:table>");
    }
    xml.push_str("</office:spreadsheet></office:body></office:document-content>");

    let manifest = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3">
 <manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>
 <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"#;

    let mut z = zip::ZipWriter::new(destino);
    let guardado =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let comprimido = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    // El mimetype va primero y sin comprimir (requisito de OpenDocument).
    z.start_file("mimetype", guardado)?;
    z.write_all(b"application/vnd.oasis.opendocument.spreadsheet")?;
    z.start_file("META-INF/manifest.xml", comprimido)?;
    z.write_all(manifest.as_bytes())?;
    z.start_file("content.xml", comprimido)?;
    z.write_all(xml.as_bytes())?;
    z.finish()?;
    Ok(())
}

pub fn exportar_ods(c: &Connection, ruta: &std::path::Path) -> Result<(), Error> {
    let t = tablas(c)?;
    let f = std::fs::File::create(ruta)?;
    escribir_ods(&t, f)
}

pub fn exportar_plantilla(ruta: &std::path::Path) -> Result<(), Error> {
    let t = crate::plantilla::tablas_plantilla();
    let f = std::fs::File::create(ruta)?;
    escribir_ods(&t, f)
}

pub fn es_formato_propio(hojas: &[Hoja]) -> bool {
    hojas.iter().any(|h| h.nombre == MARCA) && hojas.iter().any(|h| h.nombre == "Gastos")
}

/// Filas de una hoja exportada como mapas encabezado → celda.
fn filas<'a>(
    hojas: &'a [Hoja],
    nombre: &str,
) -> Result<Vec<HashMap<String, &'a crate::lectura::Celda>>, Error> {
    let h = hojas.iter().find(|h| h.nombre == nombre).ok_or_else(|| {
        Error::Formato(format!("A la exportación le falta la hoja \"{nombre}\"."))
    })?;
    let max_fila = h.celdas.keys().map(|&(f, _)| f).max().unwrap_or(0);
    let encabezados: HashMap<u32, String> = h
        .celdas
        .range((0, 0)..(1, 0))
        .map(|(&(_, c), celda)| (c, celda.texto.trim().to_string()))
        .collect();
    let mut v = Vec::new();
    for f in 1..=max_fila {
        let fila: HashMap<String, &crate::lectura::Celda> = h
            .celdas
            .range((f, 0)..(f + 1, 0))
            .filter_map(|(&(_, c), celda)| Some((encabezados.get(&c)?.clone(), celda)))
            .collect();
        if !fila.is_empty() {
            v.push(fila);
        }
    }
    Ok(v)
}

struct Fila<'a>(HashMap<String, &'a crate::lectura::Celda>);

impl Fila<'_> {
    fn dec(&self, k: &str) -> Result<Option<Decimal>, Error> {
        match self.0.get(k).and_then(|c| c.valor.as_deref()) {
            Some(v) => Decimal::from_str(v)
                .map(Some)
                .map_err(|_| Error::Formato(format!("Valor inválido en la columna {k}: {v}"))),
            None => Ok(None),
        }
    }
    fn entero(&self, k: &str) -> Result<Option<i64>, Error> {
        Ok(self.dec(k)?.and_then(|d| i64::try_from(d).ok()))
    }
    fn req(&self, k: &str) -> Result<i64, Error> {
        self.entero(k)?
            .ok_or_else(|| Error::Formato(format!("Falta la columna {k} en una fila.")))
    }
    fn txt(&self, k: &str) -> String {
        self.0.get(k).map(|c| c.texto.clone()).unwrap_or_default()
    }
    fn txt_opc(&self, k: &str) -> Option<String> {
        Some(self.txt(k)).filter(|t| !t.is_empty())
    }
    fn bool(&self, k: &str) -> Result<bool, Error> {
        Ok(self.entero(k)?.unwrap_or(0) != 0)
    }
}

/// Reemplaza todos los datos por los de una exportación propia y verifica los totales de la
/// hoja `Resumen`. Se ejecuta dentro de la transacción del importador.
pub fn restaurar(c: &Connection, hojas: &[Hoja], inf: &mut Informe) -> Result<(), Error> {
    let leer = |n: &str| -> Result<Vec<Fila>, Error> {
        Ok(filas(hojas, n)?.into_iter().map(Fila).collect())
    };
    repo::vaciar_todo(c)?;
    let mut log = |origen: String, detalle: String| {
        inf.log.push(LineaLog {
            origen,
            accion: Accion::Importada,
            detalle,
        })
    };
    for f in leer("Estados")? {
        c.execute(
            "INSERT INTO estados (id, nombre, cuenta_pagado, orden) VALUES (?1, ?2, ?3, ?4)",
            params![
                f.req("id")?,
                f.txt("nombre"),
                f.bool("cuenta_pagado")?,
                f.entero("orden")?.unwrap_or(0)
            ],
        )?;
    }
    for (hoja, tabla) in [("Categorias", "categorias"), ("Fuentes", "fuentes_ingreso")] {
        for f in leer(hoja)? {
            c.execute(
                &format!("INSERT INTO {tabla} (id, nombre) VALUES (?1, ?2)"),
                params![f.req("id")?, f.txt("nombre")],
            )?;
        }
    }
    let mut conteos = Conteos::default();
    for f in leer("Meses")? {
        c.execute(
            "INSERT INTO meses (id, anio, mes, cerrado) VALUES (?1, ?2, ?3, ?4)",
            params![
                f.req("id")?,
                f.req("anio")?,
                f.req("mes")?,
                f.bool("cerrado")?
            ],
        )?;
        conteos.meses += 1;
    }
    for f in leer("Deudas")? {
        let d = Deuda {
            id: Some(f.req("id")?),
            nombre: f.txt("nombre"),
            total: f.dec("total")?,
            ajuste: f.dec("ajuste")?.unwrap_or_default(),
            saldada: f.bool("saldada")?,
            notas: f.txt("notas"),
            origen: f.txt_opc("origen"),
        };
        repo::restaurar_deuda(c, &d, &[])?;
        log(format!("Deudas!{}", d.nombre), "deuda".into());
        conteos.deudas += 1;
    }
    for f in leer("Tarjetas")? {
        let dia = |k: &str| -> Result<Option<u8>, Error> {
            Ok(f.entero(k)?.and_then(|d| u8::try_from(d).ok()))
        };
        let t = Tarjeta {
            id: Some(f.req("id")?),
            nombre: f.txt("nombre"),
            cupo: f.dec("cupo")?,
            saldo: f.dec("saldo")?,
            dia_corte: dia("dia_corte")?,
            dia_pago: dia("dia_pago")?,
            origen: f.txt_opc("origen"),
        };
        repo::restaurar_tarjeta(c, &t, &[])?;
        conteos.tarjetas += 1;
    }
    for f in leer("Gastos")? {
        let g = Gasto {
            id: Some(f.req("id")?),
            mes_id: f.req("mes_id")?,
            concepto: f.txt("concepto"),
            monto: f.dec("monto")?.unwrap_or_default(),
            estado_id: f.req("estado_id")?,
            categoria_id: f.entero("categoria_id")?,
            deuda_id: f.entero("deuda_id")?,
            tarjeta_id: f.entero("tarjeta_id")?,
            desglose: f.txt_opc("desglose"),
            nota: f.txt("nota"),
            fuera_de_bloque: f.bool("fuera_de_bloque")?,
            orden: f.entero("orden")?.unwrap_or(0),
            origen: f.txt_opc("origen"),
        };
        g.validar()?;
        repo::restaurar_gasto(c, &g)?;
        conteos.gastos += 1;
    }
    for f in leer("Ingresos")? {
        let i = Ingreso {
            id: Some(f.req("id")?),
            mes_id: f.req("mes_id")?,
            fuente_id: f.req("fuente_id")?,
            monto: f.dec("monto")?.unwrap_or_default(),
            desglose: f.txt_opc("desglose"),
            nota: f.txt("nota"),
            origen: f.txt_opc("origen"),
        };
        repo::restaurar_ingreso(c, &i)?;
        conteos.ingresos += 1;
    }
    log(
        "exportación".into(),
        format!(
            "{} meses, {} gastos, {} ingresos restaurados",
            conteos.meses, conteos.gastos, conteos.ingresos
        ),
    );

    // Verificación contra la hoja Resumen.
    let estados = repo::listar_estados(c)?;
    let mut r = Resultado::default();
    for f in leer("Resumen")? {
        let (anio, mes) = (f.req("anio")?, f.req("mes")?);
        let Some(m) = repo::mes_por_periodo(c, anio as i32, mes as u32)?.and_then(|m| m.id) else {
            continue;
        };
        let t = totales_mes(
            &repo::gastos_de_mes(c, m)?,
            &repo::ingresos_de_mes(c, m)?,
            |e| estados.iter().any(|x| x.id == Some(e) && x.cuenta_pagado),
        );
        for (k, app) in [
            ("ingreso", t.ingreso),
            ("total", t.total),
            ("pagado", t.pagado),
            ("restante", t.restante),
            ("restante_pagado", t.restante_pagado),
        ] {
            let hoja = f.dec(k)?;
            r.comparados += 1;
            if hoja != Some(app) {
                r.diferencias.push(Diferencia {
                    concepto: format!("{k} {mes}/{anio}"),
                    hoja,
                    app: Some(app),
                });
            }
        }
    }
    inf.conteos = conteos;
    inf.conciliacion = r;
    Ok(())
}
