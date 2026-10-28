//! Lector de `.ods`: recorre `content.xml` y devuelve cada celda con su texto, el valor exacto
//! de `office:value` (como texto, sin pasar por `f64`) y su fórmula.

use crate::Error;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Celda {
    pub texto: String,
    /// `office:value` (números/moneda) tal como está en el archivo.
    pub valor: Option<String>,
    pub tipo: Option<String>,
    /// Fórmula sin el prefijo `of:` (p. ej. `=+[.C8]-[.B8]`).
    pub formula: Option<String>,
    /// Columnas combinadas (`table:number-columns-spanned`).
    pub span: u32,
}

impl Celda {
    pub fn vacia(&self) -> bool {
        self.texto.trim().is_empty() && self.valor.is_none() && self.formula.is_none()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Hoja {
    pub nombre: String,
    /// Clave: (fila, columna), ambas desde 0.
    pub celdas: BTreeMap<(u32, u32), Celda>,
}

impl Hoja {
    pub fn celda(&self, fila: u32, col: u32) -> Option<&Celda> {
        self.celdas.get(&(fila, col))
    }

    pub fn texto(&self, fila: u32, col: u32) -> &str {
        self.celda(fila, col).map(|c| c.texto.trim()).unwrap_or("")
    }
}

/// Nombre de columna: 0 → `A`, 25 → `Z`, 26 → `AA`.
pub fn nombre_columna(col: u32) -> String {
    let mut n = col + 1;
    let mut s = Vec::new();
    while n > 0 {
        let r = (n - 1) % 26;
        s.push(b'A' + r as u8);
        n = (n - 1) / 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

/// Referencia de celda en notación de hoja de cálculo: (162, 11) → `L163`.
pub fn ref_celda(fila: u32, col: u32) -> String {
    format!("{}{}", nombre_columna(col), fila + 1)
}

fn atributo(e: &BytesStart, nombre: &str) -> Result<Option<String>, Error> {
    for a in e.attributes() {
        let a = a.map_err(|x| Error::Formato(format!("XML inválido: {x}")))?;
        if a.key.as_ref() == nombre {
            let v = a
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .map_err(|x| Error::Formato(format!("XML inválido: {x}")))?;
            return Ok(Some(v.into_owned()));
        }
    }
    Ok(None)
}

fn numero(e: &BytesStart, nombre: &str) -> Result<u32, Error> {
    Ok(atributo(e, nombre)?
        .and_then(|v| v.parse().ok())
        .unwrap_or(1))
}

/// Límite de repeticiones que se expanden: las filas/celdas vacías de relleno pueden repetirse
/// un millón de veces; con contenido nunca pasa de unas pocas.
const MAX_REPETIR: u32 = 1000;

/// Lee todas las hojas de un `.ods`.
pub fn leer_libro(bytes: &[u8]) -> Result<Vec<Hoja>, Error> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| Error::Formato("El archivo no es un .ods válido (no es un ZIP).".into()))?;
    let mut xml = String::new();
    zip.by_name("content.xml")
        .map_err(|_| Error::Formato("El archivo no es un .ods válido (falta content.xml).".into()))?
        .read_to_string(&mut xml)
        .map_err(|e| Error::Formato(format!("No se pudo leer content.xml: {e}")))?;
    leer_content(&xml)
}

struct CeldaEnCurso {
    celda: Celda,
    repetir: u32,
    parrafos: u32,
}

pub fn leer_content(xml: &str) -> Result<Vec<Hoja>, Error> {
    let mut r = Reader::from_str(xml);
    let mut hojas: Vec<Hoja> = Vec::new();
    let mut fila: u32 = 0;
    let mut col: u32 = 0;
    let mut rep_fila: u32 = 1;
    let mut actual: Option<CeldaEnCurso> = None;
    let mut en_nota: u32 = 0;

    let error_xml = |e: quick_xml::Error| Error::Formato(format!("XML inválido: {e}"));

    loop {
        let ev = r.read_event().map_err(error_xml)?;
        let vacio = matches!(ev, Event::Empty(_));
        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => match e.name().as_ref() {
                "table:table" => {
                    hojas.push(Hoja {
                        nombre: atributo(e, "table:name")?.unwrap_or_default(),
                        ..Default::default()
                    });
                    fila = 0;
                }
                "table:table-row" => {
                    rep_fila = numero(e, "table:number-rows-repeated")?;
                    col = 0;
                    if vacio {
                        fila += rep_fila;
                    }
                }
                "table:table-cell" | "table:covered-table-cell" => {
                    let celda = Celda {
                        texto: String::new(),
                        valor: atributo(e, "office:value")?
                            .or(atributo(e, "office:date-value")?)
                            .or(atributo(e, "office:boolean-value")?),
                        tipo: atributo(e, "office:value-type")?,
                        formula: atributo(e, "table:formula")?
                            .map(|f| f.strip_prefix("of:").unwrap_or(&f).to_string()),
                        span: numero(e, "table:number-columns-spanned")?,
                    };
                    let en_curso = CeldaEnCurso {
                        celda,
                        repetir: numero(e, "table:number-columns-repeated")?,
                        parrafos: 0,
                    };
                    if vacio {
                        guardar(&mut hojas, fila, &mut col, rep_fila, en_curso);
                    } else {
                        actual = Some(en_curso);
                    }
                }
                "office:annotation" if !vacio => en_nota += 1,
                "text:p" if en_nota == 0 => {
                    if let Some(a) = actual.as_mut() {
                        if a.parrafos > 0 {
                            a.celda.texto.push('\n');
                        }
                        a.parrafos += 1;
                    }
                }
                "text:s" if en_nota == 0 => {
                    if let Some(a) = actual.as_mut() {
                        let n = numero(e, "text:c")? as usize;
                        a.celda.texto.push_str(&" ".repeat(n));
                    }
                }
                "text:tab" if en_nota == 0 => {
                    if let Some(a) = actual.as_mut() {
                        a.celda.texto.push('\t');
                    }
                }
                "text:line-break" if en_nota == 0 => {
                    if let Some(a) = actual.as_mut() {
                        a.celda.texto.push('\n');
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                if let (Some(a), 0) = (actual.as_mut(), en_nota) {
                    a.celda.texto.push_str(&t.xml10_content());
                }
            }
            Event::GeneralRef(g) => {
                if let (Some(a), 0) = (actual.as_mut(), en_nota) {
                    let c = if g.is_char_ref() {
                        g.resolve_char_ref().ok().flatten()
                    } else {
                        match g.as_ref() {
                            "amp" => Some('&'),
                            "lt" => Some('<'),
                            "gt" => Some('>'),
                            "quot" => Some('"'),
                            "apos" => Some('\''),
                            _ => None,
                        }
                    };
                    if let Some(c) = c {
                        a.celda.texto.push(c);
                    }
                }
            }
            Event::End(e) => match e.name().as_ref() {
                "office:annotation" => en_nota = en_nota.saturating_sub(1),
                "table:table-cell" | "table:covered-table-cell" => {
                    if let Some(a) = actual.take() {
                        guardar(&mut hojas, fila, &mut col, rep_fila, a);
                    }
                }
                "table:table-row" => fila += rep_fila,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(hojas)
}

fn guardar(hojas: &mut [Hoja], fila: u32, col: &mut u32, rep_fila: u32, a: CeldaEnCurso) {
    let Some(hoja) = hojas.last_mut() else {
        return;
    };
    if !a.celda.vacia() && a.repetir <= MAX_REPETIR && rep_fila <= MAX_REPETIR {
        for df in 0..rep_fila {
            for dc in 0..a.repetir {
                hoja.celdas.insert((fila + df, *col + dc), a.celda.clone());
            }
        }
    }
    *col += a.repetir;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columnas() {
        assert_eq!(nombre_columna(0), "A");
        assert_eq!(nombre_columna(11), "L");
        assert_eq!(nombre_columna(33), "AH");
        assert_eq!(ref_celda(162, 11), "L163");
    }

    #[test]
    fn lee_celdas_repetidas_y_texto() {
        let xml = r#"<office:document-content><office:body><office:spreadsheet>
            <table:table table:name="H">
              <table:table-row table:number-rows-repeated="2"><table:table-cell table:number-columns-repeated="5"/></table:table-row>
              <table:table-row>
                <table:table-cell table:number-columns-repeated="2"/>
                <table:table-cell office:value-type="float" office:value="12345.3000000003" table:formula="of:=+[.L72]-[.L71]"><text:p>$ 12.345,30</text:p></table:table-cell>
                <table:table-cell office:value-type="string"><text:p>Donde Ana&apos;s<text:s text:c="2"/>x</text:p><office:annotation><text:p>nota</text:p></office:annotation></table:table-cell>
              </table:table-row>
            </table:table></office:spreadsheet></office:body></office:document-content>"#;
        let h = leer_content(xml).unwrap();
        let c = h[0].celda(2, 2).unwrap();
        assert_eq!(c.valor.as_deref(), Some("12345.3000000003"));
        assert_eq!(c.formula.as_deref(), Some("=+[.L72]-[.L71]"));
        assert_eq!(h[0].texto(2, 3), "Donde Ana's  x");
        assert_eq!(h[0].celdas.len(), 2);
    }
}
