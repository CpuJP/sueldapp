//! Conversión de las vistas del núcleo a los tipos de la interfaz Slint, y paginación.

use crate::estado::Estado;
use crate::vistas::{GastoVista, Monto};
use crate::{Cifra, Fila, Pagina};
use slint::{ModelRc, SharedString, VecModel};
use sueldapp_dominio::paginacion::Direccion;

pub fn s(t: impl AsRef<str>) -> SharedString {
    SharedString::from(t.as_ref())
}

pub fn modelo<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(v))
}

pub fn textos<I: IntoIterator<Item = S>, S: AsRef<str>>(v: I) -> ModelRc<SharedString> {
    modelo(v.into_iter().map(s).collect())
}

pub fn cifra(etiqueta: &str, m: &Monto, nota: &str) -> Cifra {
    Cifra {
        etiqueta: s(etiqueta),
        valor: s(&m.texto),
        nota: s(nota),
        negativo: m.negativo,
    }
}

pub fn cifra_texto(etiqueta: &str, valor: &str, nota: &str, negativo: bool) -> Cifra {
    Cifra {
        etiqueta: s(etiqueta),
        valor: s(valor),
        nota: s(nota),
        negativo,
    }
}

/// Tipo visual del estado: 0 pendiente, 1 pagado, 2 no, 3 aplazado.
pub fn tipo_estado(g: &GastoVista) -> i32 {
    if g.pagado {
        return 1;
    }
    match sueldapp_dominio::texto::normalizar(&g.estado).as_str() {
        "no" => 2,
        "aplazado" => 3,
        _ => 0,
    }
}

pub fn fila(g: &GastoVista) -> Fila {
    let mut et = Vec::new();
    if let Some(c) = &g.categoria {
        et.push(c.clone());
    }
    if let Some(d) = &g.deuda {
        et.push(format!("Deuda: {d}"));
    }
    if let Some(t) = &g.tarjeta {
        et.push(format!("Tarjeta: {t}"));
    }
    if g.gasto.fuera_de_bloque {
        et.push("No cuenta en el total".into());
    }
    if let Some(d) = &g.gasto.desglose {
        et.push(format!("Desglose {d}"));
    }
    Fila {
        id: g.gasto.id.unwrap_or(0) as i32,
        titulo: s(&g.gasto.concepto),
        etiquetas: textos(et),
        monto: s(&g.monto_texto),
        negativo: false,
        estado: s(if g.estado.is_empty() {
            "Sin estado"
        } else {
            &g.estado
        }),
        tipo: tipo_estado(g),
        periodo: s(&g.periodo),
    }
}

pub fn pagina_vacia() -> Pagina {
    Pagina {
        desde: 0,
        hasta: 0,
        cantidad: 0,
        anterior: false,
        siguiente: false,
    }
}

/// Paginación de una lista ya cargada (listas cortas). Devuelve el trozo y la página.
pub fn paginar_local<T: Clone>(est: &mut Estado, clave: &str, items: &[T]) -> (Vec<T>, Pagina) {
    let tam = est.tam();
    let inicio = est.inicios.entry(clave.to_string()).or_insert(0);
    if *inicio >= items.len() {
        *inicio = items.len().saturating_sub(1) / tam * tam;
    }
    *inicio -= *inicio % tam;
    let fin = (*inicio + tam).min(items.len());
    let trozo = items[*inicio..fin].to_vec();
    let p = Pagina {
        desde: if items.is_empty() {
            0
        } else {
            *inicio as i32 + 1
        },
        hasta: fin as i32,
        cantidad: items.len() as i32,
        anterior: *inicio > 0,
        siguiente: fin < items.len(),
    };
    (trozo, p)
}

pub fn mover_local(est: &mut Estado, clave: &str, siguiente: bool) {
    let tam = est.tam();
    let i = est.inicios.entry(clave.to_string()).or_insert(0);
    *i = if siguiente {
        *i + tam
    } else {
        i.saturating_sub(tam)
    };
}

/// Lista de gastos paginada por cursor desde el núcleo. Guarda los gastos visibles para poder
/// abrirlos en el formulario por id.
pub fn gastos_paginados(
    est: &mut Estado,
    clave: &str,
    filtro: crate::vistas::Filtro,
) -> Result<(Vec<Fila>, Pagina, crate::vistas::Busqueda), String> {
    let (cursor, direccion) = est
        .cursores
        .get(clave)
        .cloned()
        .unwrap_or((None, Direccion::Siguiente));
    let mut f = filtro;
    f.cursor = cursor.clone();
    f.direccion = Some(direccion);
    f.limite = Some(est.tam());
    let mut r = crate::vistas::buscar_gastos(&est.c, f.clone())?;
    if r.gastos.is_empty() && cursor.is_some() {
        est.cursores.remove(clave);
        f.cursor = None;
        f.direccion = None;
        r = crate::vistas::buscar_gastos(&est.c, f)?;
    }
    let p = Pagina {
        desde: r.desde as i32,
        hasta: r.hasta as i32,
        cantidad: r.cantidad as i32,
        anterior: r.cursor_anterior.is_some(),
        siguiente: r.cursor_siguiente.is_some(),
    };
    est.ultimas.insert(
        clave.to_string(),
        (r.cursor_anterior.clone(), r.cursor_siguiente.clone()),
    );
    for g in &r.gastos {
        if let Some(id) = g.gasto.id {
            est.visibles.insert(id, g.gasto.clone());
        }
    }
    let filas = r.gastos.iter().map(fila).collect();
    Ok((filas, p, r))
}

pub fn mover_cursor(est: &mut Estado, clave: &str, siguiente: bool) {
    if let Some((ant, sig)) = est.ultimas.get(clave).cloned() {
        let c = if siguiente { sig } else { ant };
        est.cursores.insert(
            clave.to_string(),
            (
                c,
                if siguiente {
                    Direccion::Siguiente
                } else {
                    Direccion::Anterior
                },
            ),
        );
    }
}
