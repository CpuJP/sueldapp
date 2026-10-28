//! Comandos que llama la interfaz (ver `specs/001-finanzas-escritorio/contracts/comandos.md`).
//! Los montos salen como `{valor, texto}`: la interfaz nunca formatea ni calcula dinero.

use crate::Base;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::MutexGuard;
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
use tauri::State;
use tauri_plugin_dialog::DialogExt;

type R<T> = Result<T, String>;

fn base<'a>(b: &'a State<Base>) -> R<MutexGuard<'a, Connection>> {
    b.0.lock()
        .map_err(|_| "La base de datos quedó bloqueada; reinicia la app.".to_string())
}

fn e(x: impl std::fmt::Display) -> String {
    x.to_string()
}

#[derive(Serialize)]
pub struct Monto {
    valor: Decimal,
    texto: String,
    negativo: bool,
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

#[derive(Serialize)]
pub struct Totales {
    ingreso: Monto,
    total: Monto,
    pagado: Monto,
    pendiente: Monto,
    restante: Monto,
    restante_pagado: Monto,
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
            anio: periodo.0,
            mes: periodo.1,
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

#[derive(Serialize)]
pub struct GastoVista {
    #[serde(flatten)]
    gasto: Gasto,
    monto_texto: String,
    pagado: bool,
    estado: String,
    periodo: String,
    anio: i32,
    mes: u32,
    deuda: Option<String>,
    tarjeta: Option<String>,
    categoria: Option<String>,
}

#[derive(Serialize)]
pub struct IngresoVista {
    #[serde(flatten)]
    ingreso: Ingreso,
    monto_texto: String,
    fuente: String,
}

// ---------- Config y catálogos ----------

#[derive(Serialize)]
pub struct Config {
    /// `SUELDAPP_TEMA` (claro/oscuro) para capturas; `None` = seguir al sistema.
    tema: Option<String>,
    demo: bool,
    /// Modo de capturas de pantalla (solo desarrollo).
    capturas: bool,
    base: String,
    vacia: bool,
}

#[tauri::command]
pub fn config(b: State<Base>) -> R<Config> {
    let c = base(&b)?;
    Ok(Config {
        tema: std::env::var("SUELDAPP_TEMA").ok(),
        demo: std::env::var_os("SUELDAPP_DEMO").is_some(),
        capturas: std::env::var_os("SUELDAPP_CAPTURAS").is_some(),
        base: sueldapp_almacen::ruta_por_defecto().display().to_string(),
        vacia: repo::listar_meses(&c).map_err(e)?.is_empty(),
    })
}

#[derive(Serialize)]
pub struct Catalogos {
    estados: Vec<Estado>,
    categorias: Vec<Item>,
    fuentes: Vec<Item>,
    deudas: Vec<Item>,
    tarjetas: Vec<Item>,
}

#[tauri::command]
pub fn catalogos(b: State<Base>) -> R<Catalogos> {
    let c = base(&b)?;
    Ok(Catalogos {
        estados: repo::listar_estados(&c).map_err(e)?,
        categorias: repo::listar_items(&c, Catalogo::Categorias).map_err(e)?,
        fuentes: repo::listar_items(&c, Catalogo::Fuentes).map_err(e)?,
        deudas: repo::listar_deudas(&c)
            .map_err(e)?
            .into_iter()
            .map(|d| Item {
                id: d.id,
                nombre: d.nombre,
            })
            .collect(),
        tarjetas: repo::listar_tarjetas(&c)
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

#[tauri::command]
pub fn listar_meses(b: State<Base>) -> R<Vec<Mes>> {
    repo::listar_meses(&*base(&b)?).map_err(e)
}

#[tauri::command]
pub fn guardar_mes(b: State<Base>, mes: Mes) -> R<Mes> {
    repo::guardar_mes(&*base(&b)?, &mes).map_err(e)
}

#[tauri::command]
pub fn borrar_mes(b: State<Base>, id: i64) -> R<MesCompleto> {
    repo::borrar_mes(&*base(&b)?, id).map_err(e)
}

#[tauri::command]
pub fn restaurar_mes(b: State<Base>, mes: MesCompleto) -> R<()> {
    repo::restaurar_mes(&*base(&b)?, &mes).map_err(e)
}

#[derive(Serialize)]
pub struct ResumenMes {
    mes: Option<Mes>,
    nombre: String,
    totales: Totales,
    gastos: Vec<GastoVista>,
    ingresos: Vec<IngresoVista>,
    proximos: Vec<GastoVista>,
}

#[tauri::command]
pub fn resumen_mes(b: State<Base>, anio: i32, mes: u32) -> R<ResumenMes> {
    let c = base(&b)?;
    let ctx = Contexto::cargar(&c)?;
    let m = repo::mes_por_periodo(&c, anio, mes).map_err(e)?;
    let (gastos, ingresos) = match m.as_ref().and_then(|m| m.id) {
        Some(id) => (
            repo::gastos_de_mes(&c, id).map_err(e)?,
            repo::ingresos_de_mes(&c, id).map_err(e)?,
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
        gastos: gastos.into_iter().map(|g| ctx.gasto(g)).collect(),
        ingresos: ingresos.into_iter().map(|i| ctx.ingreso(i)).collect(),
        proximos,
    })
}

#[derive(Serialize)]
pub struct MesDelAnio {
    mes: u32,
    nombre: String,
    existe: bool,
    totales: Totales,
}

#[derive(Serialize)]
pub struct ResumenAnio {
    anio: i32,
    totales: Totales,
    meses: Vec<MesDelAnio>,
}

#[tauri::command]
pub fn resumen_anio(b: State<Base>, anio: i32) -> R<ResumenAnio> {
    let c = base(&b)?;
    let ctx = Contexto::cargar(&c)?;
    let meses = repo::listar_meses(&c).map_err(e)?;
    let mut lista = Vec::new();
    let mut todos = Vec::new();
    for mes in 1..=12 {
        let m = meses.iter().find(|m| m.anio == anio && m.mes == mes);
        let t = match m.and_then(|m| m.id) {
            Some(id) => totales_mes(
                &repo::gastos_de_mes(&c, id).map_err(e)?,
                &repo::ingresos_de_mes(&c, id).map_err(e)?,
                |x| ctx.pagado(x),
            ),
            None => TotalesMes::default(),
        };
        lista.push(MesDelAnio {
            mes,
            nombre: NOMBRES_MES[mes as usize - 1].to_string(),
            existe: m.is_some(),
            totales: totales(&t),
        });
        todos.push(t);
    }
    Ok(ResumenAnio {
        anio,
        totales: totales(&sumar(&todos)),
        meses: lista,
    })
}

// ---------- Gastos e ingresos ----------

#[derive(Deserialize, Default)]
pub struct Filtro {
    texto: Option<String>,
    anio: Option<i32>,
    mes: Option<u32>,
    estado_id: Option<i64>,
    categoria_id: Option<i64>,
    deuda_id: Option<i64>,
    tarjeta_id: Option<i64>,
    /// `"pagados"` o `"pendientes"`.
    pago: Option<String>,
    /// Cursor devuelto por la página anterior (`cursor_siguiente` o `cursor_anterior`).
    cursor: Option<String>,
    direccion: Option<Direccion>,
    /// Elementos por página (10 por defecto).
    limite: Option<usize>,
}

#[derive(Serialize)]
pub struct Busqueda {
    gastos: Vec<GastoVista>,
    /// Suma y cantidad de todos los resultados, no solo de la página.
    total: Monto,
    cantidad: usize,
    desde: usize,
    hasta: usize,
    cursor_siguiente: Option<String>,
    cursor_anterior: Option<String>,
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

#[tauri::command]
pub fn buscar_gastos(b: State<Base>, filtro: Filtro) -> R<Busqueda> {
    let c = base(&b)?;
    let ctx = Contexto::cargar(&c)?;
    let texto = filtro.texto.unwrap_or_default();
    let mut v: Vec<Gasto> = repo::todos_gastos(&c)
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
#[derive(Deserialize)]
pub struct GastoEntrada {
    id: Option<i64>,
    anio: i32,
    mes: u32,
    concepto: String,
    monto: String,
    estado_id: i64,
    categoria_id: Option<i64>,
    deuda_id: Option<i64>,
    tarjeta_id: Option<i64>,
    nota: String,
    fuera_de_bloque: bool,
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

#[tauri::command]
pub fn guardar_gasto(b: State<Base>, gasto: GastoEntrada) -> R<GastoVista> {
    let c = base(&b)?;
    let previo = gasto
        .id
        .map(|id| repo::gasto(&c, id))
        .transpose()
        .map_err(e)?;
    let g = Gasto {
        id: gasto.id,
        mes_id: mes_id(&c, gasto.anio, gasto.mes)?,
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
    let g = repo::guardar_gasto(&c, &g).map_err(e)?;
    Ok(Contexto::cargar(&c)?.gasto(g))
}

#[tauri::command]
pub fn borrar_gasto(b: State<Base>, id: i64) -> R<Gasto> {
    repo::borrar_gasto(&*base(&b)?, id).map_err(e)
}

#[tauri::command]
pub fn restaurar_gasto(b: State<Base>, gasto: Gasto) -> R<Gasto> {
    repo::restaurar_gasto(&*base(&b)?, &gasto).map_err(e)
}

#[derive(Deserialize)]
pub struct IngresoEntrada {
    id: Option<i64>,
    anio: i32,
    mes: u32,
    fuente_id: i64,
    monto: String,
    nota: String,
}

#[tauri::command]
pub fn guardar_ingreso(b: State<Base>, ingreso: IngresoEntrada) -> R<IngresoVista> {
    let c = base(&b)?;
    let previo = ingreso
        .id
        .map(|id| repo::ingreso(&c, id))
        .transpose()
        .map_err(e)?;
    let monto = leer_monto(&ingreso.monto).map_err(e)?;
    let i = Ingreso {
        id: ingreso.id,
        mes_id: mes_id(&c, ingreso.anio, ingreso.mes)?,
        fuente_id: ingreso.fuente_id,
        monto,
        desglose: previo
            .as_ref()
            .filter(|p| p.monto == monto)
            .and_then(|p| p.desglose.clone()),
        nota: ingreso.nota,
        origen: previo.and_then(|p| p.origen),
    };
    let i = repo::guardar_ingreso(&c, &i).map_err(e)?;
    Ok(Contexto::cargar(&c)?.ingreso(i))
}

#[tauri::command]
pub fn borrar_ingreso(b: State<Base>, id: i64) -> R<Ingreso> {
    repo::borrar_ingreso(&*base(&b)?, id).map_err(e)
}

#[tauri::command]
pub fn restaurar_ingreso(b: State<Base>, ingreso: Ingreso) -> R<Ingreso> {
    repo::restaurar_ingreso(&*base(&b)?, &ingreso).map_err(e)
}

// ---------- Deudas ----------

#[derive(Serialize)]
pub struct DeudaVista {
    #[serde(flatten)]
    deuda: Deuda,
    total_texto: Option<String>,
    ajuste_texto: String,
    pagado: Monto,
    pendiente: Option<Monto>,
    progreso: Option<Decimal>,
    cuotas_pagadas: usize,
    cuotas_registradas: usize,
    fin_estimado: Option<String>,
    cuotas: Vec<GastoVista>,
}

#[tauri::command]
pub fn listar_deudas(b: State<Base>) -> R<Vec<DeudaVista>> {
    let c = base(&b)?;
    let ctx = Contexto::cargar(&c)?;
    let gastos = repo::todos_gastos(&c).map_err(e)?;
    let mut v = Vec::new();
    for d in repo::listar_deudas(&c).map_err(e)? {
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
            ajuste_texto: formatear(d.ajuste),
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

#[derive(Deserialize)]
pub struct DeudaEntrada {
    id: Option<i64>,
    nombre: String,
    total: Option<String>,
    ajuste: Option<String>,
    saldada: bool,
    notas: String,
}

#[tauri::command]
pub fn guardar_deuda(b: State<Base>, deuda: DeudaEntrada) -> R<Deuda> {
    let c = base(&b)?;
    let previo = deuda
        .id
        .map(|id| repo::deuda(&c, id))
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
    repo::guardar_deuda(&c, &d).map_err(e)
}

#[derive(Serialize, Deserialize)]
pub struct Borrado<T> {
    registro: T,
    vinculados: Vec<i64>,
}

#[tauri::command]
pub fn borrar_deuda(b: State<Base>, id: i64) -> R<Borrado<Deuda>> {
    let (registro, vinculados) = repo::borrar_deuda(&*base(&b)?, id).map_err(e)?;
    Ok(Borrado {
        registro,
        vinculados,
    })
}

#[tauri::command]
pub fn restaurar_deuda(b: State<Base>, borrado: Borrado<Deuda>) -> R<Deuda> {
    repo::restaurar_deuda(&*base(&b)?, &borrado.registro, &borrado.vinculados).map_err(e)
}

// ---------- Tarjetas ----------

#[derive(Serialize)]
pub struct TarjetaVista {
    #[serde(flatten)]
    tarjeta: Tarjeta,
    cupo_texto: Option<String>,
    saldo_texto: Option<String>,
    disponible: Option<Monto>,
    total_pagado: Monto,
    pagos_registrados: usize,
    pagos: Vec<GastoVista>,
}

#[tauri::command]
pub fn listar_tarjetas(b: State<Base>) -> R<Vec<TarjetaVista>> {
    let c = base(&b)?;
    let ctx = Contexto::cargar(&c)?;
    let gastos = repo::todos_gastos(&c).map_err(e)?;
    let mut v = Vec::new();
    for t in repo::listar_tarjetas(&c).map_err(e)? {
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

#[derive(Deserialize)]
pub struct TarjetaEntrada {
    id: Option<i64>,
    nombre: String,
    cupo: Option<String>,
    saldo: Option<String>,
    dia_corte: Option<u8>,
    dia_pago: Option<u8>,
}

#[tauri::command]
pub fn guardar_tarjeta(b: State<Base>, tarjeta: TarjetaEntrada) -> R<Tarjeta> {
    let c = base(&b)?;
    let previo = tarjeta
        .id
        .map(|id| repo::tarjeta(&c, id))
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
    repo::guardar_tarjeta(&c, &t).map_err(e)
}

#[tauri::command]
pub fn borrar_tarjeta(b: State<Base>, id: i64) -> R<Borrado<Tarjeta>> {
    let (registro, vinculados) = repo::borrar_tarjeta(&*base(&b)?, id).map_err(e)?;
    Ok(Borrado {
        registro,
        vinculados,
    })
}

#[tauri::command]
pub fn restaurar_tarjeta(b: State<Base>, borrado: Borrado<Tarjeta>) -> R<Tarjeta> {
    repo::restaurar_tarjeta(&*base(&b)?, &borrado.registro, &borrado.vinculados).map_err(e)
}

// ---------- Catálogos ----------

#[tauri::command]
pub fn guardar_estado(b: State<Base>, estado: Estado) -> R<Estado> {
    repo::guardar_estado(&*base(&b)?, &estado).map_err(e)
}

#[tauri::command]
pub fn borrar_estado(b: State<Base>, id: i64, reasignar_a: Option<i64>) -> R<()> {
    repo::borrar_estado(&*base(&b)?, id, reasignar_a).map_err(e)
}

#[tauri::command]
pub fn guardar_item(b: State<Base>, tipo: Catalogo, item: Item) -> R<Item> {
    repo::guardar_item(&*base(&b)?, tipo, &item).map_err(e)
}

#[tauri::command]
pub fn borrar_item(b: State<Base>, tipo: Catalogo, id: i64, reasignar_a: Option<i64>) -> R<()> {
    repo::borrar_item(&*base(&b)?, tipo, id, reasignar_a).map_err(e)
}

// ---------- Importar y exportar ----------

/// Abre el diálogo y, si se elige un archivo, lo importa. `None` = cancelado.
#[tauri::command]
pub async fn importar_ods(
    app: tauri::AppHandle,
    b: State<'_, Base>,
    forzar: bool,
) -> R<Option<sueldapp_ods::importar::Informe>> {
    let Some(ruta) = app
        .dialog()
        .file()
        .set_title("Elige la hoja de cálculo")
        .add_filter("Hoja de cálculo OpenDocument", &["ods"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let ruta = ruta.into_path().map_err(e)?;
    let mut c = base(&b)?;
    sueldapp_ods::importar::importar_archivo(&mut c, &ruta, forzar)
        .map(Some)
        .map_err(e)
}

#[tauri::command]
pub async fn exportar_ods(app: tauri::AppHandle, b: State<'_, Base>) -> R<Option<String>> {
    let Some(ruta) = app
        .dialog()
        .file()
        .set_title("Guardar exportación")
        .set_file_name("sueldapp-exportacion.ods")
        .add_filter("Hoja de cálculo OpenDocument", &["ods"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let mut ruta = ruta.into_path().map_err(e)?;
    if ruta.extension().is_none() {
        ruta.set_extension("ods");
    }
    sueldapp_ods::escritura::exportar_ods(&*base(&b)?, &ruta).map_err(e)?;
    Ok(Some(ruta.display().to_string()))
}

#[tauri::command]
pub async fn exportar_csv(app: tauri::AppHandle, b: State<'_, Base>) -> R<Option<Vec<String>>> {
    let Some(carpeta) = app
        .dialog()
        .file()
        .set_title("Elige la carpeta para los CSV")
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let carpeta = carpeta.into_path().map_err(e)?;
    let rutas = sueldapp_ods::csv::exportar_csv(&*base(&b)?, &carpeta).map_err(e)?;
    Ok(Some(
        rutas.iter().map(|r| r.display().to_string()).collect(),
    ))
}

/// Guarda la plantilla para llenar a mano. `None` = cancelado.
#[tauri::command]
pub async fn descargar_plantilla(app: tauri::AppHandle, b: State<'_, Base>) -> R<Option<String>> {
    let Some(ruta) = app
        .dialog()
        .file()
        .set_title("Guardar plantilla para llenar")
        .set_file_name("sueldapp-plantilla.ods")
        .add_filter("Hoja de cálculo OpenDocument", &["ods"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let mut ruta = ruta.into_path().map_err(e)?;
    if ruta.extension().is_none() {
        ruta.set_extension("ods");
    }
    sueldapp_ods::escritura::exportar_plantilla(&*base(&b)?, &ruta).map_err(e)?;
    Ok(Some(ruta.display().to_string()))
}

#[tauri::command]
pub fn historial_importaciones(b: State<Base>) -> R<Vec<repo::Importacion>> {
    repo::listar_importaciones(&*base(&b)?).map_err(e)
}

// ---------- Tablero ----------

#[derive(Serialize)]
pub struct Porcentaje {
    valor: Decimal,
    texto: String,
}

fn porcentaje(p: Option<Decimal>) -> Option<Porcentaje> {
    p.map(|v| Porcentaje {
        valor: v,
        texto: format!("{} %", v.to_string().replace('.', ",")),
    })
}

#[derive(Serialize)]
pub struct MesTablero {
    anio: i32,
    mes: u32,
    nombre: String,
    corto: String,
    ingreso: Monto,
    gasto: Monto,
    pagado: Monto,
    balance: Monto,
    acumulado: Monto,
    tasa_ahorro: Option<Porcentaje>,
}

#[derive(Serialize)]
pub struct GrupoVista {
    nombre: String,
    monto: Monto,
    veces: usize,
    porcentaje: Option<Porcentaje>,
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

#[derive(Serialize)]
pub struct MesValor {
    nombre: String,
    monto: Monto,
}

#[derive(Serialize)]
pub struct Kpis {
    meses: usize,
    ingreso: Monto,
    gasto: Monto,
    pagado: Monto,
    por_pagar: Monto,
    balance: Monto,
    sobrante: Monto,
    faltante: Monto,
    meses_con_sobrante: usize,
    meses_con_faltante: usize,
    promedio_ingreso: Monto,
    promedio_gasto: Monto,
    promedio_balance: Monto,
    tasa_ahorro: Option<Porcentaje>,
    mejor_mes: Option<MesValor>,
    peor_mes: Option<MesValor>,
    mes_mayor_gasto: Option<MesValor>,
}

#[derive(Serialize)]
pub struct DeudaTablero {
    nombre: String,
    pendiente: Monto,
    progreso: Option<Decimal>,
}

#[derive(Serialize)]
pub struct Tablero {
    desde: Option<String>,
    hasta: Option<String>,
    kpis: Kpis,
    meses: Vec<MesTablero>,
    por_categoria: Vec<GrupoVista>,
    por_destino: Vec<GrupoVista>,
    por_estado: Vec<GrupoVista>,
    por_fuente: Vec<GrupoVista>,
    top_conceptos: Vec<GrupoVista>,
    mayores_gastos: Vec<GastoVista>,
    deuda_pendiente: Monto,
    deuda_pagada: Monto,
    deudas: Vec<DeudaTablero>,
}

fn corto((anio, mes): Periodo) -> String {
    let n = NOMBRES_MES.get(mes as usize - 1).copied().unwrap_or("?");
    format!("{} {:02}", &n[..3], anio % 100)
}

/// Estadísticas de los meses entre `desde` y `hasta` (incluidos), como `[anio, mes]`.
#[tauri::command]
pub fn tablero(b: State<Base>, desde: Option<(i32, u32)>, hasta: Option<(i32, u32)>) -> R<Tablero> {
    use sueldapp_dominio::estadisticas::{acumulado, agrupar, indicadores};
    use sueldapp_dominio::texto::{etiqueta_concepto, raiz_concepto};
    let c = base(&b)?;
    let ctx = Contexto::cargar(&c)?;
    let en_rango = |p: Periodo| desde.is_none_or(|d| p >= d) && hasta.is_none_or(|h| p <= h);
    let meses: Vec<Mes> = repo::listar_meses(&c)
        .map_err(e)?
        .into_iter()
        .filter(|m| en_rango((m.anio, m.mes)))
        .collect();
    let ids: std::collections::HashSet<i64> = meses.iter().filter_map(|m| m.id).collect();
    let gastos: Vec<Gasto> = repo::todos_gastos(&c)
        .map_err(e)?
        .into_iter()
        .filter(|g| ids.contains(&g.mes_id) && !g.fuera_de_bloque)
        .collect();
    let ingresos: Vec<Ingreso> = repo::todos_ingresos(&c)
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
    let todos = repo::todos_gastos(&c).map_err(e)?;
    let mut deudas = Vec::new();
    let (mut pendiente, mut pagada) = (Decimal::ZERO, Decimal::ZERO);
    for d in repo::listar_deudas(&c).map_err(e)? {
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
            pagado: monto(k.pagado),
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
                anio: p.0,
                mes: p.1,
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
