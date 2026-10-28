//! Repositorios CRUD. Cada `guardar_*` crea (sin id) o actualiza (con id). Si el registro no
//! tiene id pero sí `origen`, se actualiza el que ya tenga ese origen: así la importación es
//! idempotente. `restaurar_*` vuelve a insertar un registro borrado con su id (deshacer).

use crate::{Error, Resultado};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use sueldapp_dominio::{Decimal, Deuda, Estado, Gasto, Ingreso, Item, Mes, Tarjeta};

fn dec(f: &Row, i: usize) -> rusqlite::Result<Decimal> {
    let s: String = f.get(i)?;
    Decimal::from_str(&s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn dec_opc(f: &Row, i: usize) -> rusqlite::Result<Option<Decimal>> {
    let s: Option<String> = f.get(i)?;
    s.map(|s| {
        Decimal::from_str(&s).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e))
        })
    })
    .transpose()
}

fn texto(d: Decimal) -> String {
    d.normalize().to_string()
}

fn texto_opc(d: Option<Decimal>) -> Option<String> {
    d.map(texto)
}

/// Id a actualizar: el propio, o el del registro con el mismo origen.
fn resolver_id(
    c: &Connection,
    tabla: &str,
    id: Option<i64>,
    origen: &Option<String>,
) -> Resultado<Option<i64>> {
    if id.is_some() {
        return Ok(id);
    }
    match origen {
        Some(o) => Ok(c
            .query_row(
                &format!("SELECT id FROM {tabla} WHERE origen = ?1"),
                [o],
                |f| f.get(0),
            )
            .optional()?),
        None => Ok(None),
    }
}

fn exigir_cambio(n: usize) -> Resultado<()> {
    if n == 0 {
        Err(Error::NoEncontrado)
    } else {
        Ok(())
    }
}

// ---------- Meses ----------

fn fila_mes(f: &Row) -> rusqlite::Result<Mes> {
    Ok(Mes {
        id: f.get(0)?,
        anio: f.get(1)?,
        mes: f.get(2)?,
        cerrado: f.get(3)?,
    })
}

pub fn listar_meses(c: &Connection) -> Resultado<Vec<Mes>> {
    let mut s = c.prepare("SELECT id, anio, mes, cerrado FROM meses ORDER BY anio, mes")?;
    let v = s.query_map([], fila_mes)?.collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn mes(c: &Connection, id: i64) -> Resultado<Mes> {
    c.query_row(
        "SELECT id, anio, mes, cerrado FROM meses WHERE id = ?1",
        [id],
        fila_mes,
    )
    .optional()?
    .ok_or(Error::NoEncontrado)
}

pub fn mes_por_periodo(c: &Connection, anio: i32, mes: u32) -> Resultado<Option<Mes>> {
    Ok(c.query_row(
        "SELECT id, anio, mes, cerrado FROM meses WHERE anio = ?1 AND mes = ?2",
        params![anio, mes],
        fila_mes,
    )
    .optional()?)
}

pub fn guardar_mes(c: &Connection, m: &Mes) -> Resultado<Mes> {
    m.validar()?;
    let id =
        m.id.or(mes_por_periodo(c, m.anio, m.mes)?.and_then(|x| x.id));
    let r = match id {
        Some(id) => c
            .execute(
                "UPDATE meses SET anio = ?1, mes = ?2, cerrado = ?3 WHERE id = ?4",
                params![m.anio, m.mes, m.cerrado, id],
            )
            .map(|n| (n, id)),
        None => c
            .execute(
                "INSERT INTO meses (anio, mes, cerrado) VALUES (?1, ?2, ?3)",
                params![m.anio, m.mes, m.cerrado],
            )
            .map(|n| (n, c.last_insert_rowid())),
    };
    match r {
        Ok((n, id)) => {
            exigir_cambio(n)?;
            mes(c, id)
        }
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(Error::Validacion("Ya existe ese mes.".into()))
        }
        Err(e) => Err(e.into()),
    }
}

/// Un mes con todo su contenido, para poder deshacer su borrado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MesCompleto {
    pub mes: Mes,
    pub gastos: Vec<Gasto>,
    pub ingresos: Vec<Ingreso>,
}

pub fn borrar_mes(c: &Connection, id: i64) -> Resultado<MesCompleto> {
    let completo = MesCompleto {
        mes: mes(c, id)?,
        gastos: gastos_de_mes(c, id)?,
        ingresos: ingresos_de_mes(c, id)?,
    };
    c.execute("DELETE FROM meses WHERE id = ?1", [id])?;
    Ok(completo)
}

pub fn restaurar_mes(c: &Connection, m: &MesCompleto) -> Resultado<()> {
    c.execute(
        "INSERT INTO meses (id, anio, mes, cerrado) VALUES (?1, ?2, ?3, ?4)",
        params![m.mes.id, m.mes.anio, m.mes.mes, m.mes.cerrado],
    )?;
    for g in &m.gastos {
        insertar_gasto(c, g, g.id)?;
    }
    for i in &m.ingresos {
        insertar_ingreso(c, i, i.id)?;
    }
    Ok(())
}

// ---------- Gastos ----------

const COLS_GASTO: &str = "id, mes_id, concepto, monto, estado_id, categoria_id, deuda_id, \
    tarjeta_id, desglose, nota, fuera_de_bloque, orden, origen";

fn fila_gasto(f: &Row) -> rusqlite::Result<Gasto> {
    Ok(Gasto {
        id: f.get(0)?,
        mes_id: f.get(1)?,
        concepto: f.get(2)?,
        monto: dec(f, 3)?,
        estado_id: f.get(4)?,
        categoria_id: f.get(5)?,
        deuda_id: f.get(6)?,
        tarjeta_id: f.get(7)?,
        desglose: f.get(8)?,
        nota: f.get(9)?,
        fuera_de_bloque: f.get(10)?,
        orden: f.get(11)?,
        origen: f.get(12)?,
    })
}

fn consultar_gastos(
    c: &Connection,
    filtro: &str,
    p: &[&dyn rusqlite::ToSql],
) -> Resultado<Vec<Gasto>> {
    let mut s = c.prepare(&format!(
        "SELECT {COLS_GASTO} FROM gastos {filtro} ORDER BY mes_id, orden, id"
    ))?;
    let v = s.query_map(p, fila_gasto)?.collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn gastos_de_mes(c: &Connection, mes_id: i64) -> Resultado<Vec<Gasto>> {
    consultar_gastos(c, "WHERE mes_id = ?1", &[&mes_id])
}

pub fn todos_gastos(c: &Connection) -> Resultado<Vec<Gasto>> {
    consultar_gastos(c, "", &[])
}

pub fn gasto(c: &Connection, id: i64) -> Resultado<Gasto> {
    consultar_gastos(c, "WHERE id = ?1", &[&id])?
        .pop()
        .ok_or(Error::NoEncontrado)
}

fn insertar_gasto(c: &Connection, g: &Gasto, id: Option<i64>) -> Resultado<i64> {
    c.execute(
        &format!(
            "INSERT INTO gastos ({COLS_GASTO}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)"
        ),
        params![
            id,
            g.mes_id,
            g.concepto.trim(),
            texto(g.monto),
            g.estado_id,
            g.categoria_id,
            g.deuda_id,
            g.tarjeta_id,
            g.desglose,
            g.nota,
            g.fuera_de_bloque,
            g.orden,
            g.origen
        ],
    )?;
    Ok(c.last_insert_rowid())
}

pub fn guardar_gasto(c: &Connection, g: &Gasto) -> Resultado<Gasto> {
    g.validar()?;
    let id = match resolver_id(c, "gastos", g.id, &g.origen)? {
        Some(id) => {
            let n = c.execute(
                "UPDATE gastos SET mes_id=?1, concepto=?2, monto=?3, estado_id=?4, categoria_id=?5, \
                 deuda_id=?6, tarjeta_id=?7, desglose=?8, nota=?9, fuera_de_bloque=?10, orden=?11, \
                 origen=?12 WHERE id=?13",
                params![
                    g.mes_id,
                    g.concepto.trim(),
                    texto(g.monto),
                    g.estado_id,
                    g.categoria_id,
                    g.deuda_id,
                    g.tarjeta_id,
                    g.desglose,
                    g.nota,
                    g.fuera_de_bloque,
                    g.orden,
                    g.origen,
                    id
                ],
            )?;
            exigir_cambio(n)?;
            id
        }
        None => insertar_gasto(c, g, None)?,
    };
    gasto(c, id)
}

pub fn borrar_gasto(c: &Connection, id: i64) -> Resultado<Gasto> {
    let g = gasto(c, id)?;
    c.execute("DELETE FROM gastos WHERE id = ?1", [id])?;
    Ok(g)
}

pub fn restaurar_gasto(c: &Connection, g: &Gasto) -> Resultado<Gasto> {
    let id = insertar_gasto(c, g, g.id)?;
    gasto(c, id)
}

// ---------- Ingresos ----------

const COLS_INGRESO: &str = "id, mes_id, fuente_id, monto, desglose, nota, origen";

fn fila_ingreso(f: &Row) -> rusqlite::Result<Ingreso> {
    Ok(Ingreso {
        id: f.get(0)?,
        mes_id: f.get(1)?,
        fuente_id: f.get(2)?,
        monto: dec(f, 3)?,
        desglose: f.get(4)?,
        nota: f.get(5)?,
        origen: f.get(6)?,
    })
}

fn consultar_ingresos(
    c: &Connection,
    filtro: &str,
    p: &[&dyn rusqlite::ToSql],
) -> Resultado<Vec<Ingreso>> {
    let mut s = c.prepare(&format!(
        "SELECT {COLS_INGRESO} FROM ingresos {filtro} ORDER BY mes_id, fuente_id, id"
    ))?;
    let v = s.query_map(p, fila_ingreso)?.collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn ingresos_de_mes(c: &Connection, mes_id: i64) -> Resultado<Vec<Ingreso>> {
    consultar_ingresos(c, "WHERE mes_id = ?1", &[&mes_id])
}

pub fn todos_ingresos(c: &Connection) -> Resultado<Vec<Ingreso>> {
    consultar_ingresos(c, "", &[])
}

pub fn ingreso(c: &Connection, id: i64) -> Resultado<Ingreso> {
    consultar_ingresos(c, "WHERE id = ?1", &[&id])?
        .pop()
        .ok_or(Error::NoEncontrado)
}

fn insertar_ingreso(c: &Connection, i: &Ingreso, id: Option<i64>) -> Resultado<i64> {
    c.execute(
        &format!("INSERT INTO ingresos ({COLS_INGRESO}) VALUES (?1,?2,?3,?4,?5,?6,?7)"),
        params![
            id,
            i.mes_id,
            i.fuente_id,
            texto(i.monto),
            i.desglose,
            i.nota,
            i.origen
        ],
    )?;
    Ok(c.last_insert_rowid())
}

pub fn guardar_ingreso(c: &Connection, i: &Ingreso) -> Resultado<Ingreso> {
    i.validar()?;
    let id = match resolver_id(c, "ingresos", i.id, &i.origen)? {
        Some(id) => {
            let n = c.execute(
                "UPDATE ingresos SET mes_id=?1, fuente_id=?2, monto=?3, desglose=?4, nota=?5, \
                 origen=?6 WHERE id=?7",
                params![
                    i.mes_id,
                    i.fuente_id,
                    texto(i.monto),
                    i.desglose,
                    i.nota,
                    i.origen,
                    id
                ],
            )?;
            exigir_cambio(n)?;
            id
        }
        None => insertar_ingreso(c, i, None)?,
    };
    ingreso(c, id)
}

pub fn borrar_ingreso(c: &Connection, id: i64) -> Resultado<Ingreso> {
    let i = ingreso(c, id)?;
    c.execute("DELETE FROM ingresos WHERE id = ?1", [id])?;
    Ok(i)
}

pub fn restaurar_ingreso(c: &Connection, i: &Ingreso) -> Resultado<Ingreso> {
    let id = insertar_ingreso(c, i, i.id)?;
    ingreso(c, id)
}

// ---------- Deudas ----------

const COLS_DEUDA: &str = "id, nombre, total, ajuste, saldada, notas, origen";

fn fila_deuda(f: &Row) -> rusqlite::Result<Deuda> {
    Ok(Deuda {
        id: f.get(0)?,
        nombre: f.get(1)?,
        total: dec_opc(f, 2)?,
        ajuste: dec(f, 3)?,
        saldada: f.get(4)?,
        notas: f.get(5)?,
        origen: f.get(6)?,
    })
}

pub fn listar_deudas(c: &Connection) -> Resultado<Vec<Deuda>> {
    let mut s = c.prepare(&format!("SELECT {COLS_DEUDA} FROM deudas ORDER BY id"))?;
    let v = s.query_map([], fila_deuda)?.collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn deuda(c: &Connection, id: i64) -> Resultado<Deuda> {
    c.query_row(
        &format!("SELECT {COLS_DEUDA} FROM deudas WHERE id = ?1"),
        [id],
        fila_deuda,
    )
    .optional()?
    .ok_or(Error::NoEncontrado)
}

fn insertar_deuda(c: &Connection, d: &Deuda, id: Option<i64>) -> Resultado<i64> {
    c.execute(
        &format!("INSERT INTO deudas ({COLS_DEUDA}) VALUES (?1,?2,?3,?4,?5,?6,?7)"),
        params![
            id,
            d.nombre.trim(),
            texto_opc(d.total),
            texto(d.ajuste),
            d.saldada,
            d.notas,
            d.origen
        ],
    )?;
    Ok(c.last_insert_rowid())
}

pub fn guardar_deuda(c: &Connection, d: &Deuda) -> Resultado<Deuda> {
    d.validar()?;
    let id = match resolver_id(c, "deudas", d.id, &d.origen)? {
        Some(id) => {
            let n = c.execute(
                "UPDATE deudas SET nombre=?1, total=?2, ajuste=?3, saldada=?4, notas=?5, origen=?6 \
                 WHERE id=?7",
                params![
                    d.nombre.trim(),
                    texto_opc(d.total),
                    texto(d.ajuste),
                    d.saldada,
                    d.notas,
                    d.origen,
                    id
                ],
            )?;
            exigir_cambio(n)?;
            id
        }
        None => insertar_deuda(c, d, None)?,
    };
    deuda(c, id)
}

/// Borra la deuda; sus cuotas quedan como gastos sin deuda. Devuelve la deuda y los ids de las
/// cuotas que tenía, para poder deshacer.
pub fn borrar_deuda(c: &Connection, id: i64) -> Resultado<(Deuda, Vec<i64>)> {
    let d = deuda(c, id)?;
    let cuotas = ids_vinculados(c, "deuda_id", id)?;
    c.execute("DELETE FROM deudas WHERE id = ?1", [id])?;
    Ok((d, cuotas))
}

pub fn restaurar_deuda(c: &Connection, d: &Deuda, cuotas: &[i64]) -> Resultado<Deuda> {
    let id = insertar_deuda(c, d, d.id)?;
    revincular(c, "deuda_id", id, cuotas)?;
    deuda(c, id)
}

fn ids_vinculados(c: &Connection, columna: &str, id: i64) -> Resultado<Vec<i64>> {
    let mut s = c.prepare(&format!("SELECT id FROM gastos WHERE {columna} = ?1"))?;
    let v = s.query_map([id], |f| f.get(0))?.collect::<Result<_, _>>()?;
    Ok(v)
}

fn revincular(c: &Connection, columna: &str, id: i64, gastos: &[i64]) -> Resultado<()> {
    for g in gastos {
        c.execute(
            &format!("UPDATE gastos SET {columna} = ?1 WHERE id = ?2"),
            [id, *g],
        )?;
    }
    Ok(())
}

// ---------- Tarjetas ----------

const COLS_TARJETA: &str = "id, nombre, cupo, saldo, dia_corte, dia_pago, origen";

fn fila_tarjeta(f: &Row) -> rusqlite::Result<Tarjeta> {
    Ok(Tarjeta {
        id: f.get(0)?,
        nombre: f.get(1)?,
        cupo: dec_opc(f, 2)?,
        saldo: dec_opc(f, 3)?,
        dia_corte: f.get(4)?,
        dia_pago: f.get(5)?,
        origen: f.get(6)?,
    })
}

pub fn listar_tarjetas(c: &Connection) -> Resultado<Vec<Tarjeta>> {
    let mut s = c.prepare(&format!(
        "SELECT {COLS_TARJETA} FROM tarjetas ORDER BY nombre"
    ))?;
    let v = s.query_map([], fila_tarjeta)?.collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn tarjeta(c: &Connection, id: i64) -> Resultado<Tarjeta> {
    c.query_row(
        &format!("SELECT {COLS_TARJETA} FROM tarjetas WHERE id = ?1"),
        [id],
        fila_tarjeta,
    )
    .optional()?
    .ok_or(Error::NoEncontrado)
}

fn insertar_tarjeta(c: &Connection, t: &Tarjeta, id: Option<i64>) -> Resultado<i64> {
    c.execute(
        &format!("INSERT INTO tarjetas ({COLS_TARJETA}) VALUES (?1,?2,?3,?4,?5,?6,?7)"),
        params![
            id,
            t.nombre.trim(),
            texto_opc(t.cupo),
            texto_opc(t.saldo),
            t.dia_corte,
            t.dia_pago,
            t.origen
        ],
    )?;
    Ok(c.last_insert_rowid())
}

pub fn guardar_tarjeta(c: &Connection, t: &Tarjeta) -> Resultado<Tarjeta> {
    t.validar()?;
    let id = match resolver_id(c, "tarjetas", t.id, &t.origen)? {
        Some(id) => {
            let n = c.execute(
                "UPDATE tarjetas SET nombre=?1, cupo=?2, saldo=?3, dia_corte=?4, dia_pago=?5, \
                 origen=?6 WHERE id=?7",
                params![
                    t.nombre.trim(),
                    texto_opc(t.cupo),
                    texto_opc(t.saldo),
                    t.dia_corte,
                    t.dia_pago,
                    t.origen,
                    id
                ],
            )?;
            exigir_cambio(n)?;
            id
        }
        None => insertar_tarjeta(c, t, None)?,
    };
    tarjeta(c, id)
}

pub fn borrar_tarjeta(c: &Connection, id: i64) -> Resultado<(Tarjeta, Vec<i64>)> {
    let t = tarjeta(c, id)?;
    let pagos = ids_vinculados(c, "tarjeta_id", id)?;
    c.execute("DELETE FROM tarjetas WHERE id = ?1", [id])?;
    Ok((t, pagos))
}

pub fn restaurar_tarjeta(c: &Connection, t: &Tarjeta, pagos: &[i64]) -> Resultado<Tarjeta> {
    let id = insertar_tarjeta(c, t, t.id)?;
    revincular(c, "tarjeta_id", id, pagos)?;
    tarjeta(c, id)
}

// ---------- Catálogos ----------

pub fn listar_estados(c: &Connection) -> Resultado<Vec<Estado>> {
    let mut s =
        c.prepare("SELECT id, nombre, cuenta_pagado, orden FROM estados ORDER BY orden, id")?;
    let v = s
        .query_map([], |f| {
            Ok(Estado {
                id: f.get(0)?,
                nombre: f.get(1)?,
                cuenta_pagado: f.get(2)?,
                orden: f.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

fn duplicado(e: rusqlite::Error) -> Error {
    match e {
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Error::Validacion("Ya existe un elemento con ese nombre.".into())
        }
        e => e.into(),
    }
}

pub fn guardar_estado(c: &Connection, e: &Estado) -> Resultado<Estado> {
    e.validar()?;
    let id = match e.id {
        Some(id) => {
            let n = c
                .execute(
                    "UPDATE estados SET nombre=?1, cuenta_pagado=?2, orden=?3 WHERE id=?4",
                    params![e.nombre.trim(), e.cuenta_pagado, e.orden, id],
                )
                .map_err(duplicado)?;
            exigir_cambio(n)?;
            id
        }
        None => {
            c.execute(
                "INSERT INTO estados (nombre, cuenta_pagado, orden) VALUES (?1, ?2, ?3)",
                params![e.nombre.trim(), e.cuenta_pagado, e.orden],
            )
            .map_err(duplicado)?;
            c.last_insert_rowid()
        }
    };
    listar_estados(c)?
        .into_iter()
        .find(|x| x.id == Some(id))
        .ok_or(Error::NoEncontrado)
}

/// Borra un estado. Si hay gastos con ese estado, exige `reasignar_a`.
pub fn borrar_estado(c: &Connection, id: i64, reasignar_a: Option<i64>) -> Resultado<()> {
    let en_uso: i64 = c.query_row(
        "SELECT COUNT(*) FROM gastos WHERE estado_id = ?1",
        [id],
        |f| f.get(0),
    )?;
    if en_uso > 0 {
        let destino = reasignar_a.filter(|d| *d != id).ok_or_else(|| {
            Error::Validacion(format!(
                "Hay {en_uso} movimientos con este estado. Elige a qué estado pasan."
            ))
        })?;
        c.execute(
            "UPDATE gastos SET estado_id = ?1 WHERE estado_id = ?2",
            [destino, id],
        )?;
    }
    exigir_cambio(c.execute("DELETE FROM estados WHERE id = ?1", [id])?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Catalogo {
    Categorias,
    Fuentes,
}

impl Catalogo {
    fn tabla(self) -> &'static str {
        match self {
            Catalogo::Categorias => "categorias",
            Catalogo::Fuentes => "fuentes_ingreso",
        }
    }
}

pub fn listar_items(c: &Connection, cat: Catalogo) -> Resultado<Vec<Item>> {
    let mut s = c.prepare(&format!(
        "SELECT id, nombre FROM {} ORDER BY id",
        cat.tabla()
    ))?;
    let v = s
        .query_map([], |f| {
            Ok(Item {
                id: f.get(0)?,
                nombre: f.get(1)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn guardar_item(c: &Connection, cat: Catalogo, i: &Item) -> Resultado<Item> {
    i.validar()?;
    let t = cat.tabla();
    let id = match i.id {
        Some(id) => {
            let n = c
                .execute(
                    &format!("UPDATE {t} SET nombre = ?1 WHERE id = ?2"),
                    params![i.nombre.trim(), id],
                )
                .map_err(duplicado)?;
            exigir_cambio(n)?;
            id
        }
        None => {
            c.execute(
                &format!("INSERT INTO {t} (nombre) VALUES (?1)"),
                [i.nombre.trim()],
            )
            .map_err(duplicado)?;
            c.last_insert_rowid()
        }
    };
    Ok(Item {
        id: Some(id),
        nombre: i.nombre.trim().to_string(),
    })
}

/// Borra un elemento. Categorías: los gastos quedan sin categoría o pasan a `reasignar_a`.
/// Fuentes: si hay ingresos, exige `reasignar_a`.
pub fn borrar_item(
    c: &Connection,
    cat: Catalogo,
    id: i64,
    reasignar_a: Option<i64>,
) -> Resultado<()> {
    let destino = reasignar_a.filter(|d| *d != id);
    match cat {
        Catalogo::Categorias => {
            c.execute(
                "UPDATE gastos SET categoria_id = ?1 WHERE categoria_id = ?2",
                params![destino, id],
            )?;
        }
        Catalogo::Fuentes => {
            let en_uso: i64 = c.query_row(
                "SELECT COUNT(*) FROM ingresos WHERE fuente_id = ?1",
                [id],
                |f| f.get(0),
            )?;
            if en_uso > 0 {
                let d = destino.ok_or_else(|| {
                    Error::Validacion(format!(
                        "Hay {en_uso} ingresos con esta fuente. Elige a qué fuente pasan."
                    ))
                })?;
                c.execute(
                    "UPDATE ingresos SET fuente_id = ?1 WHERE fuente_id = ?2",
                    [d, id],
                )?;
            }
        }
    }
    exigir_cambio(c.execute(&format!("DELETE FROM {} WHERE id = ?1", cat.tabla()), [id])?)
}

/// Id de un elemento por nombre exacto; lo crea si no existe.
pub fn id_item(c: &Connection, cat: Catalogo, nombre: &str) -> Resultado<i64> {
    let t = cat.tabla();
    if let Some(id) = c
        .query_row(
            &format!("SELECT id FROM {t} WHERE nombre = ?1"),
            [nombre],
            |f| f.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    c.execute(&format!("INSERT INTO {t} (nombre) VALUES (?1)"), [nombre])?;
    Ok(c.last_insert_rowid())
}

/// Id de un estado por nombre exacto; lo crea (no pagado) si no existe.
pub fn id_estado(c: &Connection, nombre: &str) -> Resultado<i64> {
    if let Some(id) = c
        .query_row("SELECT id FROM estados WHERE nombre = ?1", [nombre], |f| {
            f.get(0)
        })
        .optional()?
    {
        return Ok(id);
    }
    c.execute(
        "INSERT INTO estados (nombre, cuenta_pagado, orden) VALUES (?1, 0, 99)",
        [nombre],
    )?;
    Ok(c.last_insert_rowid())
}

// ---------- Importaciones ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Importacion {
    pub id: i64,
    pub fecha: String,
    pub archivo: String,
    pub huella: String,
    pub ok: bool,
    pub resumen: String,
}

pub fn registrar_importacion(
    c: &Connection,
    archivo: &str,
    huella: &str,
    ok: bool,
    resumen: &str,
    log: &str,
) -> Resultado<i64> {
    c.execute(
        "INSERT INTO importaciones (archivo, huella, ok, resumen, log) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![archivo, huella, ok, resumen, log],
    )?;
    Ok(c.last_insert_rowid())
}

pub fn listar_importaciones(c: &Connection) -> Resultado<Vec<Importacion>> {
    let mut s = c.prepare(
        "SELECT id, fecha, archivo, huella, ok, resumen FROM importaciones ORDER BY id DESC",
    )?;
    let v = s
        .query_map([], |f| {
            Ok(Importacion {
                id: f.get(0)?,
                fecha: f.get(1)?,
                archivo: f.get(2)?,
                huella: f.get(3)?,
                ok: f.get(4)?,
                resumen: f.get(5)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

/// Borra todos los datos (no los catálogos de estados/categorías/fuentes, que se reemplazan
/// aparte). Se usa al restaurar una exportación propia, siempre dentro de una transacción.
pub fn vaciar_todo(c: &Connection) -> Resultado<()> {
    c.execute_batch(
        "DELETE FROM gastos; DELETE FROM ingresos; DELETE FROM meses; DELETE FROM deudas;
         DELETE FROM tarjetas; DELETE FROM estados; DELETE FROM categorias;
         DELETE FROM fuentes_ingreso;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abrir_memoria;
    use sueldapp_dominio::Decimal;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn nuevo_gasto(mes_id: i64, origen: Option<&str>) -> Gasto {
        Gasto {
            id: None,
            mes_id,
            concepto: "Internet".into(),
            monto: d("12500.50"),
            estado_id: 1,
            categoria_id: None,
            deuda_id: None,
            tarjeta_id: None,
            desglose: Some("=12500+0.5".into()),
            nota: String::new(),
            fuera_de_bloque: false,
            orden: 0,
            origen: origen.map(String::from),
        }
    }

    fn mes_nuevo(c: &Connection) -> i64 {
        guardar_mes(
            c,
            &Mes {
                id: None,
                anio: 2026,
                mes: 10,
                cerrado: false,
            },
        )
        .unwrap()
        .id
        .unwrap()
    }

    #[test]
    fn crud_gasto_y_deshacer() {
        let c = abrir_memoria().unwrap();
        let m = mes_nuevo(&c);
        let g = guardar_gasto(&c, &nuevo_gasto(m, None)).unwrap();
        assert_eq!(g.monto, d("12500.5"));
        let mut e = g.clone();
        e.monto = d("100");
        assert_eq!(guardar_gasto(&c, &e).unwrap().monto, d("100"));
        let borrado = borrar_gasto(&c, g.id.unwrap()).unwrap();
        assert!(gastos_de_mes(&c, m).unwrap().is_empty());
        restaurar_gasto(&c, &borrado).unwrap();
        assert_eq!(gastos_de_mes(&c, m).unwrap(), vec![borrado]);
    }

    #[test]
    fn upsert_por_origen() {
        let c = abrir_memoria().unwrap();
        let m = mes_nuevo(&c);
        guardar_gasto(&c, &nuevo_gasto(m, Some("Nómina!L9"))).unwrap();
        guardar_gasto(&c, &nuevo_gasto(m, Some("Nómina!L9"))).unwrap();
        assert_eq!(todos_gastos(&c).unwrap().len(), 1);
    }

    #[test]
    fn mes_duplicado_se_reutiliza() {
        let c = abrir_memoria().unwrap();
        let a = mes_nuevo(&c);
        let b = mes_nuevo(&c);
        assert_eq!(a, b);
        assert_eq!(listar_meses(&c).unwrap().len(), 1);
    }

    #[test]
    fn borrar_mes_y_restaurar() {
        let c = abrir_memoria().unwrap();
        let m = mes_nuevo(&c);
        guardar_gasto(&c, &nuevo_gasto(m, None)).unwrap();
        let completo = borrar_mes(&c, m).unwrap();
        assert!(todos_gastos(&c).unwrap().is_empty());
        restaurar_mes(&c, &completo).unwrap();
        assert_eq!(todos_gastos(&c).unwrap().len(), 1);
    }

    #[test]
    fn borrar_deuda_conserva_cuotas() {
        let c = abrir_memoria().unwrap();
        let m = mes_nuevo(&c);
        let deu = guardar_deuda(
            &c,
            &Deuda {
                id: None,
                nombre: "PC".into(),
                total: Some(d("1000")),
                ajuste: d("0.68"),
                saldada: false,
                notas: String::new(),
                origen: None,
            },
        )
        .unwrap();
        let mut g = nuevo_gasto(m, None);
        g.deuda_id = deu.id;
        let g = guardar_gasto(&c, &g).unwrap();
        let (borrada, cuotas) = borrar_deuda(&c, deu.id.unwrap()).unwrap();
        assert_eq!(gasto(&c, g.id.unwrap()).unwrap().deuda_id, None);
        restaurar_deuda(&c, &borrada, &cuotas).unwrap();
        assert_eq!(gasto(&c, g.id.unwrap()).unwrap().deuda_id, deu.id);
        assert_eq!(deuda(&c, deu.id.unwrap()).unwrap().ajuste, d("0.68"));
    }

    #[test]
    fn borrar_estado_en_uso_exige_reasignar() {
        let c = abrir_memoria().unwrap();
        let m = mes_nuevo(&c);
        let mut g = nuevo_gasto(m, None);
        g.estado_id = 2;
        guardar_gasto(&c, &g).unwrap();
        assert!(matches!(
            borrar_estado(&c, 2, None),
            Err(Error::Validacion(_))
        ));
        borrar_estado(&c, 2, Some(1)).unwrap();
        assert_eq!(todos_gastos(&c).unwrap()[0].estado_id, 1);
    }

    #[test]
    fn catalogo_duplicado() {
        let c = abrir_memoria().unwrap();
        let r = guardar_item(
            &c,
            Catalogo::Categorias,
            &Item {
                id: None,
                nombre: "Tarjetas".into(),
            },
        );
        assert!(matches!(r, Err(Error::Validacion(_))));
        borrar_item(&c, Catalogo::Fuentes, 3, None).unwrap();
        assert_eq!(listar_items(&c, Catalogo::Fuentes).unwrap().len(), 2);
    }
}
