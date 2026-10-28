//! Vistas: datos ya calculados y formateados que muestra la interfaz. Todo el dinero sale del
//! núcleo como `Monto {valor, texto}`: la interfaz nunca calcula ni formatea dinero.

use std::collections::HashMap;
use sueldapp_almacen::repo::{self, Catalogo, MesCompleto};
use sueldapp_almacen::Connection;
use sueldapp_dominio::calculos::{
    saldo_deuda, saldo_tarjeta, sumar, totales_mes, Cuota, TotalesMes,
};
use sueldapp_dominio::formato::{formatear, leer_monto};
use sueldapp_dominio::paginacion::{paginar, Direccion};
use sueldapp_dominio::texto::contiene;
use sueldapp_dominio::{
    Decimal, Deuda, Estado, Gasto, Ingreso, Item, Mes, Periodo, Tarjeta, NOMBRES_MES,
};

type R<T> = Result<T, String>;

fn e(x: impl std::fmt::Display) -> String {
    x.to_string()
}

pub struct Monto {
    pub valor: Decimal,
    pub texto: String,
    pub negativo: bool,
}

fn monto(d: Decimal) -> Monto {
    Monto {
        valor: d,
        texto: formatear(d),
        negativo: d.is_sign_negative() && !d.is_zero(),
    }
}

fn nombre_periodo((anio, mes): Periodo) -> String {
    format!(
        "{} {anio}",
        NOMBRES_MES.get(mes as usize - 1).copied().unwrap_or("?")
    )
}

pub struct Totales {
    pub ingreso: Monto,
    pub total: Monto,
    pub pagado: Monto,
    pub pendiente: Monto,
    pub restante: Monto,
    pub restante_pagado: Monto,
}

fn totales(t: &TotalesMes) -> Totales {
    Totales {
        ingreso: monto(t.ingreso),
        total: monto(t.total),
        pagado: monto(t.pagado),
        pendiente: monto(t.total - t.pagado),
        restante: monto(t.restante),
        restante_pagado: monto(t.restante_pagado),
    }
}

/// Datos de apoyo para mostrar y filtrar: catálogos y periodos.
struct Contexto {
    estados: Vec<Estado>,
    periodos: HashMap<i64, Periodo>,
    deudas: HashMap<i64, String>,
    tarjetas: HashMap<i64, String>,
    categorias: HashMap<i64, String>,
    fuentes: HashMap<i64, String>,
}

impl Contexto {
    fn cargar(c: &Connection) -> R<Contexto> {
        let mapa = |v: Vec<Item>| {
            v.into_iter()
                .filter_map(|i| Some((i.id?, i.nombre)))
                .collect()
        };
        Ok(Contexto {
            estados: repo::listar_estados(c).map_err(e)?,
            periodos: repo::listar_meses(c)
                .map_err(e)?
                .into_iter()
                .filter_map(|m| Some((m.id?, (m.anio, m.mes))))
                .collect(),
            deudas: repo::listar_deudas(c)
                .map_err(e)?
                .into_iter()
                .filter_map(|d| Some((d.id?, d.nombre)))
                .collect(),
            tarjetas: repo::listar_tarjetas(c)
                .map_err(e)?
                .into_iter()
                .filter_map(|t| Some((t.id?, t.nombre)))
                .collect(),
            categorias: mapa(repo::listar_items(c, Catalogo::Categorias).map_err(e)?),
            fuentes: mapa(repo::listar_items(c, Catalogo::Fuentes).map_err(e)?),
        })
    }

    fn pagado(&self, estado_id: i64) -> bool {
        self.estados
            .iter()
            .any(|x| x.id == Some(estado_id) && x.cuenta_pagado)
    }

    fn periodo(&self, mes_id: i64) -> Periodo {
        self.periodos.get(&mes_id).copied().unwrap_or((0, 1))
    }

    fn gasto(&self, g: Gasto) -> GastoVista {
        let estado = self
            .estados
            .iter()
            .find(|x| x.id == Some(g.estado_id))
            .map(|x| x.nombre.clone())
            .unwrap_or_default();
        let periodo = self.periodo(g.mes_id);
        GastoVista {
            monto_texto: formatear(g.monto),
            pagado: self.pagado(g.estado_id),
            estado,
            periodo: nombre_periodo(periodo),
            deuda: g.deuda_id.and_then(|i| self.deudas.get(&i).cloned()),
            tarjeta: g.tarjeta_id.and_then(|i| self.tarjetas.get(&i).cloned()),
            categoria: g
                .categoria_id
                .and_then(|i| self.categorias.get(&i).cloned()),
            gasto: g,
        }
    }

    fn ingreso(&self, i: Ingreso) -> IngresoVista {
        IngresoVista {
            monto_texto: formatear(i.monto),
            fuente: self.fuentes.get(&i.fuente_id).cloned().unwrap_or_default(),
            ingreso: i,
        }
    }
}

pub struct GastoVista {
    pub gasto: Gasto,
    pub monto_texto: String,
    pub pagado: bool,
    pub estado: String,
    pub periodo: String,
    pub deuda: Option<String>,
    pub tarjeta: Option<String>,
    pub categoria: Option<String>,
}

pub struct IngresoVista {
    pub ingreso: Ingreso,
    pub monto_texto: String,
    pub fuente: String,
}

// ---------- Config y catálogos ----------

pub struct Catalogos {
    pub estados: Vec<Estado>,
    pub categorias: Vec<Item>,
    pub fuentes: Vec<Item>,
    pub deudas: Vec<Item>,
    pub tarjetas: Vec<Item>,
}

pub fn catalogos(c: &Connection) -> R<Catalogos> {
    Ok(Catalogos {
        estados: repo::listar_estados(c).map_err(e)?,
        categorias: repo::listar_items(c, Catalogo::Categorias).map_err(e)?,
        fuentes: repo::listar_items(c, Catalogo::Fuentes).map_err(e)?,
        deudas: repo::listar_deudas(c)
            .map_err(e)?
            .into_iter()
            .map(|d| Item {
                id: d.id,
                nombre: d.nombre,
            })
            .collect(),
        tarjetas: repo::listar_tarjetas(c)
            .map_err(e)?
            .into_iter()
            .map(|t| Item {
                id: t.id,
                nombre: t.nombre,
            })
            .collect(),
    })
}

// ---------- Meses y resúmenes ----------

pub fn guardar_mes(c: &Connection, mes: Mes) -> R<Mes> {
    repo::guardar_mes(c, &mes).map_err(e)
}

pub fn borrar_mes(c: &Connection, id: i64) -> R<MesCompleto> {
    repo::borrar_mes(c, id).map_err(e)
}

pub fn restaurar_mes(c: &Connection, mes: MesCompleto) -> R<()> {
    repo::restaurar_mes(c, &mes).map_err(e)
}

pub struct ResumenMes {
    pub mes: Option<Mes>,
    pub nombre: String,
    pub totales: Totales,
    pub ingresos: Vec<IngresoVista>,
    pub proximos: Vec<GastoVista>,
}

pub fn resumen_mes(c: &Connection, anio: i32, mes: u32) -> R<ResumenMes> {
    let ctx = Contexto::cargar(c)?;
    let m = repo::mes_por_periodo(c, anio, mes).map_err(e)?;
    let (gastos, ingresos) = match m.as_ref().and_then(|m| m.id) {
        Some(id) => (
            repo::gastos_de_mes(c, id).map_err(e)?,
            repo::ingresos_de_mes(c, id).map_err(e)?,
        ),
        None => (vec![], vec![]),
    };
    let t = totales_mes(&gastos, &ingresos, |x| ctx.pagado(x));
    let proximos = gastos
        .iter()
        .filter(|g| !ctx.pagado(g.estado_id) && !g.fuera_de_bloque)
        .cloned()
        .map(|g| ctx.gasto(g))
        .collect();
    Ok(ResumenMes {
        nombre: nombre_periodo((anio, mes)),
        mes: m,
        totales: totales(&t),
        ingresos: ingresos.into_iter().map(|i| ctx.ingreso(i)).collect(),
        proximos,
    })
}

pub struct MesDelAnio {
    pub nombre: String,
    pub existe: bool,
    pub totales: Totales,
}

pub struct ResumenAnio {
    pub totales: Totales,
    pub meses: Vec<MesDelAnio>,
}

pub fn resumen_anio(c: &Connection, anio: i32) -> R<ResumenAnio> {
    let ctx = Contexto::cargar(c)?;
    let meses = repo::listar_meses(c).map_err(e)?;
    let mut lista = Vec::new();
    let mut todos = Vec::new();
    for mes in 1..=12 {
        let m = meses.iter().find(|m| m.anio == anio && m.mes == mes);
        let t = match m.and_then(|m| m.id) {
            Some(id) => totales_mes(
                &repo::gastos_de_mes(c, id).map_err(e)?,
                &repo::ingresos_de_mes(c, id).map_err(e)?,
                |x| ctx.pagado(x),
            ),
            None => TotalesMes::default(),
        };
        lista.push(MesDelAnio {
            nombre: NOMBRES_MES[mes as usize - 1].to_string(),
            existe: m.is_some(),
            totales: totales(&t),
        });
        todos.push(t);
    }
    Ok(ResumenAnio {
        totales: totales(&sumar(&todos)),
        meses: lista,
    })
}

// ---------- Gastos e ingresos ----------

#[derive(Default, Clone)]
pub struct Filtro {
    pub texto: Option<String>,
    pub anio: Option<i32>,
    pub mes: Option<u32>,
    pub estado_id: Option<i64>,
    pub categoria_id: Option<i64>,
    pub deuda_id: Option<i64>,
    pub tarjeta_id: Option<i64>,
    /// `"pagados"` o `"pendientes"`.
    pub pago: Option<String>,
    /// Cursor devuelto por la página anterior (`cursor_siguiente` o `cursor_anterior`).
    pub cursor: Option<String>,
    pub direccion: Option<Direccion>,
    /// Elementos por página (10 por defecto).
    pub limite: Option<usize>,
}

pub struct Busqueda {
    pub gastos: Vec<GastoVista>,
    /// Suma y cantidad de todos los resultados, no solo de la página.
    pub total: Monto,
    pub cantidad: usize,
    pub desde: usize,
    pub hasta: usize,
    pub cursor_siguiente: Option<String>,
    pub cursor_anterior: Option<String>,
}

/// Clave de orden de los gastos: mes más reciente primero; dentro del mes, orden de la hoja.
type ClaveGasto = (std::cmp::Reverse<Periodo>, i64, i64);

fn cursor_texto(k: &ClaveGasto) -> String {
    let ((a, m), o, id) = (k.0 .0, k.1, k.2);
    format!("{a}:{m}:{o}:{id}")
}

fn cursor_clave(t: &str) -> R<ClaveGasto> {
    let p: Vec<&str> = t.split(':').collect();
    let error = || "Cursor de página inválido.".to_string();
    if p.len() != 4 {
        return Err(error());
    }
    Ok((
        std::cmp::Reverse((
            p[0].parse().map_err(|_| error())?,
            p[1].parse().map_err(|_| error())?,
        )),
        p[2].parse().map_err(|_| error())?,
        p[3].parse().map_err(|_| error())?,
    ))
}

pub fn buscar_gastos(c: &Connection, filtro: Filtro) -> R<Busqueda> {
    let ctx = Contexto::cargar(c)?;
    let texto = filtro.texto.unwrap_or_default();
    let mut v: Vec<Gasto> = repo::todos_gastos(c)
        .map_err(e)?
        .into_iter()
        .filter(|g| {
            let (a, m) = ctx.periodo(g.mes_id);
            (texto.trim().is_empty()
                || contiene(&g.concepto, texto.trim())
                || contiene(&g.nota, texto.trim()))
                && filtro.anio.is_none_or(|x| x == a)
                && filtro.mes.is_none_or(|x| x == m)
                && filtro.estado_id.is_none_or(|x| x == g.estado_id)
                && filtro
                    .categoria_id
                    .is_none_or(|x| Some(x) == g.categoria_id)
                && filtro.deuda_id.is_none_or(|x| Some(x) == g.deuda_id)
                && filtro.tarjeta_id.is_none_or(|x| Some(x) == g.tarjeta_id)
                && match filtro.pago.as_deref() {
                    Some("pagados") => ctx.pagado(g.estado_id),
                    Some("pendientes") => !ctx.pagado(g.estado_id),
                    _ => true,
                }
        })
        .collect();
    let clave = |g: &Gasto| -> ClaveGasto {
        (
            std::cmp::Reverse(ctx.periodo(g.mes_id)),
            g.orden,
            g.id.unwrap_or(0),
        )
    };
    v.sort_by_key(clave);
    let total = v.iter().map(|g| g.monto).sum();
    let cursor = filtro.cursor.as_deref().map(cursor_clave).transpose()?;
    let p = paginar(
        v,
        clave,
        cursor.as_ref(),
        filtro.direccion.unwrap_or_default(),
        filtro.limite.unwrap_or(10),
    );
    Ok(Busqueda {
        total: monto(total),
        cantidad: p.cantidad,
        desde: p.desde,
        hasta: p.hasta,
        cursor_siguiente: p.cursor_siguiente.as_ref().map(cursor_texto),
        cursor_anterior: p.cursor_anterior.as_ref().map(cursor_texto),
        gastos: p.elementos.into_iter().map(|g| ctx.gasto(g)).collect(),
    })
}

/// Datos de un formulario de gasto. El monto llega como lo escribió el usuario.
pub struct GastoEntrada {
    pub id: Option<i64>,
    pub anio: i32,
    pub mes: u32,
    pub concepto: String,
    pub monto: String,
    pub estado_id: i64,
    pub categoria_id: Option<i64>,
    pub deuda_id: Option<i64>,
    pub tarjeta_id: Option<i64>,
    pub nota: String,
    pub fuera_de_bloque: bool,
}

/// Id del mes; lo crea si no existe.
fn mes_id(c: &Connection, anio: i32, mes: u32) -> R<i64> {
    let m = repo::guardar_mes(
        c,
        &Mes {
            id: None,
            anio,
            mes,
            cerrado: repo::mes_por_periodo(c, anio, mes)
                .map_err(e)?
                .is_some_and(|m| m.cerrado),
        },
    )
    .map_err(e)?;
    m.id.ok_or_else(|| "mes sin id".to_string())
}

pub fn guardar_gasto(c: &Connection, gasto: GastoEntrada) -> R<GastoVista> {
    let previo = gasto
        .id
        .map(|id| repo::gasto(c, id))
        .transpose()
        .map_err(e)?;
    let g = Gasto {
        id: gasto.id,
        mes_id: mes_id(c, gasto.anio, gasto.mes)?,
        concepto: gasto.concepto,
        monto: leer_monto(&gasto.monto).map_err(e)?,
        estado_id: gasto.estado_id,
        categoria_id: gasto.categoria_id,
        deuda_id: gasto.deuda_id,
        tarjeta_id: gasto.tarjeta_id,
        // El desglose de la hoja solo vale mientras el monto no cambie.
        desglose: previo
            .as_ref()
            .filter(|p| leer_monto(&gasto.monto).is_ok_and(|m| m == p.monto))
            .and_then(|p| p.desglose.clone()),
        nota: gasto.nota,
        fuera_de_bloque: gasto.fuera_de_bloque,
        orden: previo.as_ref().map(|p| p.orden).unwrap_or(i64::MAX / 2),
        origen: previo.and_then(|p| p.origen),
    };
    let g = repo::guardar_gasto(c, &g).map_err(e)?;
    Ok(Contexto::cargar(c)?.gasto(g))
}

pub fn borrar_gasto(c: &Connection, id: i64) -> R<Gasto> {
    repo::borrar_gasto(c, id).map_err(e)
}

pub fn restaurar_gasto(c: &Connection, gasto: Gasto) -> R<Gasto> {
    repo::restaurar_gasto(c, &gasto).map_err(e)
}

pub struct IngresoEntrada {
    pub id: Option<i64>,
    pub anio: i32,
    pub mes: u32,
    pub fuente_id: i64,
    pub monto: String,
    pub nota: String,
}

pub fn guardar_ingreso(c: &Connection, ingreso: IngresoEntrada) -> R<IngresoVista> {
    let previo = ingreso
        .id
        .map(|id| repo::ingreso(c, id))
        .transpose()
        .map_err(e)?;
    let monto = leer_monto(&ingreso.monto).map_err(e)?;
    let i = Ingreso {
        id: ingreso.id,
        mes_id: mes_id(c, ingreso.anio, ingreso.mes)?,
        fuente_id: ingreso.fuente_id,
        monto,
        desglose: previo
            .as_ref()
            .filter(|p| p.monto == monto)
            .and_then(|p| p.desglose.clone()),
        nota: ingreso.nota,
        origen: previo.and_then(|p| p.origen),
    };
    let i = repo::guardar_ingreso(c, &i).map_err(e)?;
    Ok(Contexto::cargar(c)?.ingreso(i))
}

pub fn borrar_ingreso(c: &Connection, id: i64) -> R<Ingreso> {
    repo::borrar_ingreso(c, id).map_err(e)
}

pub fn restaurar_ingreso(c: &Connection, ingreso: Ingreso) -> R<Ingreso> {
    repo::restaurar_ingreso(c, &ingreso).map_err(e)
}

// ---------- Deudas ----------

pub struct DeudaVista {
    pub deuda: Deuda,
    pub total_texto: Option<String>,
    pub pagado: Monto,
    pub pendiente: Option<Monto>,
    pub progreso: Option<Decimal>,
    pub cuotas_pagadas: usize,
    pub cuotas_registradas: usize,
    pub fin_estimado: Option<String>,
    pub cuotas: Vec<GastoVista>,
}

pub fn listar_deudas(c: &Connection) -> R<Vec<DeudaVista>> {
    let ctx = Contexto::cargar(c)?;
    let gastos = repo::todos_gastos(c).map_err(e)?;
    let mut v = Vec::new();
    for d in repo::listar_deudas(c).map_err(e)? {
        let mut propias: Vec<Gasto> = gastos
            .iter()
            .filter(|g| g.deuda_id.is_some() && g.deuda_id == d.id)
            .cloned()
            .collect();
        propias.sort_by_key(|g| (ctx.periodo(g.mes_id), g.orden));
        let cuotas: Vec<Cuota> = propias
            .iter()
            .map(|g| Cuota {
                monto: g.monto,
                pagada: ctx.pagado(g.estado_id),
                periodo: ctx.periodo(g.mes_id),
            })
            .collect();
        let s = saldo_deuda(d.total, d.ajuste, &cuotas);
        v.push(DeudaVista {
            total_texto: d.total.map(formatear),
            pagado: monto(s.pagado),
            pendiente: s.pendiente.map(monto),
            progreso: s.progreso,
            cuotas_pagadas: s.cuotas_pagadas,
            cuotas_registradas: s.cuotas_registradas,
            fin_estimado: s.fin_estimado.map(nombre_periodo),
            cuotas: propias.into_iter().map(|g| ctx.gasto(g)).collect(),
            deuda: d,
        });
    }
    Ok(v)
}

fn monto_opcional(t: &Option<String>) -> R<Option<Decimal>> {
    match t.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => leer_monto(s).map(Some).map_err(e),
    }
}

pub struct DeudaEntrada {
    pub id: Option<i64>,
    pub nombre: String,
    pub total: Option<String>,
    pub ajuste: Option<String>,
    pub saldada: bool,
    pub notas: String,
}

pub fn guardar_deuda(c: &Connection, deuda: DeudaEntrada) -> R<Deuda> {
    let previo = deuda
        .id
        .map(|id| repo::deuda(c, id))
        .transpose()
        .map_err(e)?;
    let d = Deuda {
        id: deuda.id,
        nombre: deuda.nombre,
        total: monto_opcional(&deuda.total)?,
        ajuste: monto_opcional(&deuda.ajuste)?.unwrap_or_default(),
        saldada: deuda.saldada,
        notas: deuda.notas,
        origen: previo.and_then(|p| p.origen),
    };
    repo::guardar_deuda(c, &d).map_err(e)
}

pub struct Borrado<T> {
    pub registro: T,
    pub vinculados: Vec<i64>,
}

pub fn borrar_deuda(c: &Connection, id: i64) -> R<Borrado<Deuda>> {
    let (registro, vinculados) = repo::borrar_deuda(c, id).map_err(e)?;
    Ok(Borrado {
        registro,
        vinculados,
    })
}

pub fn restaurar_deuda(c: &Connection, borrado: Borrado<Deuda>) -> R<Deuda> {
    repo::restaurar_deuda(c, &borrado.registro, &borrado.vinculados).map_err(e)
}

// ---------- Tarjetas ----------

pub struct TarjetaVista {
    pub tarjeta: Tarjeta,
    pub cupo_texto: Option<String>,
    pub saldo_texto: Option<String>,
    pub disponible: Option<Monto>,
    pub total_pagado: Monto,
    pub pagos_registrados: usize,
    pub pagos: Vec<GastoVista>,
}

pub fn listar_tarjetas(c: &Connection) -> R<Vec<TarjetaVista>> {
    let ctx = Contexto::cargar(c)?;
    let gastos = repo::todos_gastos(c).map_err(e)?;
    let mut v = Vec::new();
    for t in repo::listar_tarjetas(c).map_err(e)? {
        let mut propios: Vec<Gasto> = gastos
            .iter()
            .filter(|g| g.tarjeta_id.is_some() && g.tarjeta_id == t.id)
            .cloned()
            .collect();
        propios.sort_by_key(|g| (ctx.periodo(g.mes_id), g.orden));
        let pagos: Vec<Cuota> = propios
            .iter()
            .map(|g| Cuota {
                monto: g.monto,
                pagada: ctx.pagado(g.estado_id),
                periodo: ctx.periodo(g.mes_id),
            })
            .collect();
        let s = saldo_tarjeta(&t, &pagos);
        v.push(TarjetaVista {
            cupo_texto: t.cupo.map(formatear),
            saldo_texto: t.saldo.map(formatear),
            disponible: s.disponible.map(monto),
            total_pagado: monto(s.total_pagado),
            pagos_registrados: s.pagos_registrados,
            pagos: propios.into_iter().map(|g| ctx.gasto(g)).collect(),
            tarjeta: t,
        });
    }
    Ok(v)
}

pub struct TarjetaEntrada {
    pub id: Option<i64>,
    pub nombre: String,
    pub cupo: Option<String>,
    pub saldo: Option<String>,
    pub dia_corte: Option<u8>,
    pub dia_pago: Option<u8>,
}

pub fn guardar_tarjeta(c: &Connection, tarjeta: TarjetaEntrada) -> R<Tarjeta> {
    let previo = tarjeta
        .id
        .map(|id| repo::tarjeta(c, id))
        .transpose()
        .map_err(e)?;
    let t = Tarjeta {
        id: tarjeta.id,
        nombre: tarjeta.nombre,
        cupo: monto_opcional(&tarjeta.cupo)?,
        saldo: monto_opcional(&tarjeta.saldo)?,
        dia_corte: tarjeta.dia_corte,
        dia_pago: tarjeta.dia_pago,
        origen: previo.and_then(|p| p.origen),
    };
    repo::guardar_tarjeta(c, &t).map_err(e)
}

pub fn borrar_tarjeta(c: &Connection, id: i64) -> R<Borrado<Tarjeta>> {
    let (registro, vinculados) = repo::borrar_tarjeta(c, id).map_err(e)?;
    Ok(Borrado {
        registro,
        vinculados,
    })
}

pub fn restaurar_tarjeta(c: &Connection, borrado: Borrado<Tarjeta>) -> R<Tarjeta> {
    repo::restaurar_tarjeta(c, &borrado.registro, &borrado.vinculados).map_err(e)
}

// ---------- Catálogos ----------

pub fn guardar_estado(c: &Connection, estado: Estado) -> R<Estado> {
    repo::guardar_estado(c, &estado).map_err(e)
}

pub fn borrar_estado(c: &Connection, id: i64, reasignar_a: Option<i64>) -> R<()> {
    repo::borrar_estado(c, id, reasignar_a).map_err(e)
}

pub fn guardar_item(c: &Connection, tipo: Catalogo, item: Item) -> R<Item> {
    repo::guardar_item(c, tipo, &item).map_err(e)
}

pub fn borrar_item(c: &Connection, tipo: Catalogo, id: i64, reasignar_a: Option<i64>) -> R<()> {
    repo::borrar_item(c, tipo, id, reasignar_a).map_err(e)
}

// ---------- Importar y exportar ----------

// ---------- Tablero ----------

pub struct Porcentaje {
    pub texto: String,
}

fn porcentaje(p: Option<Decimal>) -> Option<Porcentaje> {
    p.map(|v| Porcentaje {
        texto: format!("{} %", v.to_string().replace('.', ",")),
    })
}

pub struct MesTablero {
    pub nombre: String,
    pub corto: String,
    pub ingreso: Monto,
    pub gasto: Monto,
    pub pagado: Monto,
    pub balance: Monto,
    pub acumulado: Monto,
    pub tasa_ahorro: Option<Porcentaje>,
}

pub struct GrupoVista {
    pub nombre: String,
    pub monto: Monto,
    pub veces: usize,
    pub porcentaje: Option<Porcentaje>,
}

fn grupos(v: Vec<sueldapp_dominio::estadisticas::Grupo>, max: usize) -> Vec<GrupoVista> {
    v.into_iter()
        .take(max)
        .map(|g| GrupoVista {
            nombre: g.nombre,
            monto: monto(g.monto),
            veces: g.veces,
            porcentaje: porcentaje(g.porcentaje),
        })
        .collect()
}

pub struct MesValor {
    pub nombre: String,
    pub monto: Monto,
}

pub struct Kpis {
    pub meses: usize,
    pub ingreso: Monto,
    pub gasto: Monto,
    pub por_pagar: Monto,
    pub balance: Monto,
    pub sobrante: Monto,
    pub faltante: Monto,
    pub meses_con_sobrante: usize,
    pub meses_con_faltante: usize,
    pub promedio_ingreso: Monto,
    pub promedio_gasto: Monto,
    pub promedio_balance: Monto,
    pub tasa_ahorro: Option<Porcentaje>,
    pub mejor_mes: Option<MesValor>,
    pub peor_mes: Option<MesValor>,
    pub mes_mayor_gasto: Option<MesValor>,
}

pub struct DeudaTablero {
    pub nombre: String,
    pub pendiente: Monto,
    pub progreso: Option<Decimal>,
}

pub struct Tablero {
    pub desde: Option<String>,
    pub hasta: Option<String>,
    pub kpis: Kpis,
    pub meses: Vec<MesTablero>,
    pub por_categoria: Vec<GrupoVista>,
    pub por_destino: Vec<GrupoVista>,
    pub por_estado: Vec<GrupoVista>,
    pub por_fuente: Vec<GrupoVista>,
    pub top_conceptos: Vec<GrupoVista>,
    pub mayores_gastos: Vec<GastoVista>,
    pub deuda_pendiente: Monto,
    pub deuda_pagada: Monto,
    pub deudas: Vec<DeudaTablero>,
}

fn corto((anio, mes): Periodo) -> String {
    let n = NOMBRES_MES.get(mes as usize - 1).copied().unwrap_or("?");
    format!("{} {:02}", &n[..3], anio % 100)
}

/// Estadísticas de los meses entre `desde` y `hasta` (incluidos), como `[anio, mes]`.
pub fn tablero(c: &Connection, desde: Option<(i32, u32)>, hasta: Option<(i32, u32)>) -> R<Tablero> {
    use sueldapp_dominio::estadisticas::{acumulado, agrupar, indicadores};
    use sueldapp_dominio::texto::{etiqueta_concepto, raiz_concepto};
    let ctx = Contexto::cargar(c)?;
    let en_rango = |p: Periodo| desde.is_none_or(|d| p >= d) && hasta.is_none_or(|h| p <= h);
    let meses: Vec<Mes> = repo::listar_meses(c)
        .map_err(e)?
        .into_iter()
        .filter(|m| en_rango((m.anio, m.mes)))
        .collect();
    let ids: std::collections::HashSet<i64> = meses.iter().filter_map(|m| m.id).collect();
    let gastos: Vec<Gasto> = repo::todos_gastos(c)
        .map_err(e)?
        .into_iter()
        .filter(|g| ids.contains(&g.mes_id) && !g.fuera_de_bloque)
        .collect();
    let ingresos: Vec<Ingreso> = repo::todos_ingresos(c)
        .map_err(e)?
        .into_iter()
        .filter(|i| ids.contains(&i.mes_id))
        .collect();

    let mut por_mes = Vec::new();
    for m in &meses {
        let id = m.id.unwrap_or_default();
        let g: Vec<Gasto> = gastos.iter().filter(|x| x.mes_id == id).cloned().collect();
        let i: Vec<Ingreso> = ingresos
            .iter()
            .filter(|x| x.mes_id == id)
            .cloned()
            .collect();
        por_mes.push(((m.anio, m.mes), totales_mes(&g, &i, |x| ctx.pagado(x))));
    }
    let k = indicadores(&por_mes);
    let acum = acumulado(&por_mes.iter().map(|(_, t)| t.restante).collect::<Vec<_>>());
    let mes_valor = |x: Option<(Periodo, Decimal)>| {
        x.map(|(p, v)| MesValor {
            nombre: nombre_periodo(p),
            monto: monto(v),
        })
    };

    let nombre_o = |m: &HashMap<i64, String>, id: Option<i64>, defecto: &str| {
        id.and_then(|i| m.get(&i).cloned())
            .unwrap_or_else(|| defecto.to_string())
    };
    let por_categoria = agrupar(
        gastos.iter(),
        |g| {
            let n = nombre_o(&ctx.categorias, g.categoria_id, "Sin categoría");
            (n.clone(), n)
        },
        |g| g.monto,
    );
    let por_destino = agrupar(
        gastos.iter(),
        |g| {
            let n = if g.deuda_id.is_some() {
                "Cuotas de deudas"
            } else if g.tarjeta_id.is_some() {
                "Tarjetas de crédito"
            } else {
                "Otros gastos"
            };
            (n.into(), n.into())
        },
        |g| g.monto,
    );
    let estado_de = |id: i64| {
        ctx.estados
            .iter()
            .find(|x| x.id == Some(id))
            .map(|x| x.nombre.clone())
            .unwrap_or_default()
    };
    let por_estado = agrupar(
        gastos.iter(),
        |g| {
            let n = estado_de(g.estado_id);
            (n.clone(), n)
        },
        |g| g.monto,
    );
    let por_fuente = agrupar(
        ingresos.iter(),
        |i| {
            let n = nombre_o(&ctx.fuentes, Some(i.fuente_id), "Sin fuente");
            (n.clone(), n)
        },
        |i| i.monto,
    );
    let top_conceptos = agrupar(
        gastos.iter(),
        |g| (raiz_concepto(&g.concepto), etiqueta_concepto(&g.concepto)),
        |g| g.monto,
    );
    let mut mayores = gastos.clone();
    mayores.sort_by_key(|g| std::cmp::Reverse(g.monto));

    // Deudas: estado actual (no depende del rango).
    let todos = repo::todos_gastos(c).map_err(e)?;
    let mut deudas = Vec::new();
    let (mut pendiente, mut pagada) = (Decimal::ZERO, Decimal::ZERO);
    for d in repo::listar_deudas(c).map_err(e)? {
        let cuotas: Vec<Cuota> = todos
            .iter()
            .filter(|g| g.deuda_id.is_some() && g.deuda_id == d.id)
            .map(|g| Cuota {
                monto: g.monto,
                pagada: ctx.pagado(g.estado_id),
                periodo: ctx.periodo(g.mes_id),
            })
            .collect();
        let s = saldo_deuda(d.total, d.ajuste, &cuotas);
        pagada += s.pagado;
        if let Some(p) = s.pendiente.filter(|p| p.is_sign_positive() && !p.is_zero()) {
            pendiente += p;
            if !d.saldada {
                deudas.push(DeudaTablero {
                    nombre: d.nombre,
                    pendiente: monto(p),
                    progreso: s.progreso,
                });
            }
        }
    }
    deudas.sort_by_key(|d| std::cmp::Reverse(d.pendiente.valor));

    Ok(Tablero {
        desde: por_mes.first().map(|(p, _)| nombre_periodo(*p)),
        hasta: por_mes.last().map(|(p, _)| nombre_periodo(*p)),
        kpis: Kpis {
            meses: k.meses,
            ingreso: monto(k.ingreso),
            gasto: monto(k.gasto),
            por_pagar: monto(k.gasto - k.pagado),
            balance: monto(k.balance),
            sobrante: monto(k.sobrante),
            faltante: monto(k.faltante),
            meses_con_sobrante: k.meses_con_sobrante,
            meses_con_faltante: k.meses_con_faltante,
            promedio_ingreso: monto(k.promedio_ingreso),
            promedio_gasto: monto(k.promedio_gasto),
            promedio_balance: monto(k.promedio_balance),
            tasa_ahorro: porcentaje(k.tasa_ahorro),
            mejor_mes: mes_valor(k.mejor_mes),
            peor_mes: mes_valor(k.peor_mes),
            mes_mayor_gasto: mes_valor(k.mes_mayor_gasto),
        },
        meses: por_mes
            .iter()
            .zip(acum)
            .map(|((p, t), a)| MesTablero {
                nombre: nombre_periodo(*p),
                corto: corto(*p),
                ingreso: monto(t.ingreso),
                gasto: monto(t.total),
                pagado: monto(t.pagado),
                balance: monto(t.restante),
                acumulado: monto(a),
                tasa_ahorro: porcentaje(sueldapp_dominio::estadisticas::porcentaje(
                    t.restante, t.ingreso,
                )),
            })
            .collect(),
        por_categoria: grupos(por_categoria, 8),
        por_destino: grupos(por_destino, 3),
        por_estado: grupos(por_estado, 8),
        por_fuente: grupos(por_fuente, 8),
        top_conceptos: grupos(top_conceptos, 10),
        mayores_gastos: mayores.into_iter().take(5).map(|g| ctx.gasto(g)).collect(),
        deuda_pendiente: monto(pendiente),
        deuda_pagada: monto(pagada),
        deudas,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sueldapp_almacen::abrir_memoria;
    use sueldapp_dominio::paginacion::Direccion;

    fn gasto(
        c: &Connection,
        concepto: &str,
        monto: &str,
        mes: u32,
        deuda_id: Option<i64>,
    ) -> GastoVista {
        guardar_gasto(
            c,
            GastoEntrada {
                id: None,
                anio: 2026,
                mes,
                concepto: concepto.into(),
                monto: monto.into(),
                estado_id: 2,
                categoria_id: None,
                deuda_id,
                tarjeta_id: None,
                nota: String::new(),
                fuera_de_bloque: false,
            },
        )
        .unwrap()
    }

    #[test]
    fn gastos_resumen_y_deshacer() {
        let c = abrir_memoria().unwrap();
        let g = gasto(&c, "Arriendo", "950.000", 10, None);
        assert_eq!(g.monto_texto, "$ 950.000,00");
        guardar_ingreso(
            &c,
            IngresoEntrada {
                id: None,
                anio: 2026,
                mes: 10,
                fuente_id: 1,
                monto: "3.200.000".into(),
                nota: String::new(),
            },
        )
        .unwrap();
        let r = resumen_mes(&c, 2026, 10).unwrap();
        assert_eq!(r.totales.restante.texto, "$ 2.250.000,00");
        assert_eq!(r.proximos.len(), 1);
        // Monto inválido: error en lenguaje sencillo, nada se guarda.
        let mal = guardar_gasto(
            &c,
            GastoEntrada {
                id: None,
                anio: 2026,
                mes: 10,
                concepto: "X".into(),
                monto: "abc".into(),
                estado_id: 2,
                categoria_id: None,
                deuda_id: None,
                tarjeta_id: None,
                nota: String::new(),
                fuera_de_bloque: false,
            },
        );
        assert!(mal.is_err());
        let borrado = borrar_gasto(&c, g.gasto.id.unwrap()).unwrap();
        assert!(resumen_mes(&c, 2026, 10).unwrap().proximos.is_empty());
        restaurar_gasto(&c, borrado).unwrap();
        assert_eq!(resumen_mes(&c, 2026, 10).unwrap().proximos.len(), 1);
    }

    #[test]
    fn paginacion_por_cursor_del_mas_reciente() {
        let c = abrir_memoria().unwrap();
        for mes in 1..=12 {
            gasto(&c, &format!("G{mes}"), "1000", mes, None);
        }
        let p1 = buscar_gastos(
            &c,
            Filtro {
                limite: Some(5),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(p1.cantidad, 12);
        assert_eq!(p1.gastos[0].gasto.concepto, "G12");
        let p2 = buscar_gastos(
            &c,
            Filtro {
                limite: Some(5),
                cursor: p1.cursor_siguiente.clone(),
                direccion: Some(Direccion::Siguiente),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!((p2.desde, p2.hasta), (6, 10));
        let vuelta = buscar_gastos(
            &c,
            Filtro {
                limite: Some(5),
                cursor: p2.cursor_anterior.clone(),
                direccion: Some(Direccion::Anterior),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(vuelta.gastos[0].gasto.concepto, "G12");
    }

    #[test]
    fn deudas_y_tablero() {
        let c = abrir_memoria().unwrap();
        let d = guardar_deuda(
            &c,
            DeudaEntrada {
                id: None,
                nombre: "Portátil".into(),
                total: Some("1.000.000".into()),
                ajuste: None,
                saldada: false,
                notas: String::new(),
            },
        )
        .unwrap();
        let g = gasto(&c, "Portátil – Cuota 1", "250.000", 9, d.id);
        let mut pagada = sueldapp_almacen::repo::gasto(&c, g.gasto.id.unwrap()).unwrap();
        pagada.estado_id = 1;
        sueldapp_almacen::repo::guardar_gasto(&c, &pagada).unwrap();
        let v = listar_deudas(&c).unwrap();
        assert_eq!(v[0].pagado.texto, "$ 250.000,00");
        assert_eq!(v[0].pendiente.as_ref().unwrap().texto, "$ 750.000,00");
        let t = tablero(&c, Some((2026, 1)), Some((2026, 12))).unwrap();
        assert_eq!(t.meses.len(), 1);
        assert_eq!(t.kpis.gasto.texto, "$ 250.000,00");
        assert_eq!(t.deuda_pendiente.texto, "$ 750.000,00");
    }
}
