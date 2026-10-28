//! Paginación por cursor (keyset): la página siguiente empieza justo después de la clave del
//! último elemento visto, y la anterior termina justo antes de la clave del primero. Así una
//! página no se corre si se crean o borran registros en otras páginas.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direccion {
    #[default]
    Siguiente,
    Anterior,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pagina<T, K> {
    pub elementos: Vec<T>,
    /// Posición (desde 1) del primer y último elemento de la página; 0 si está vacía.
    pub desde: usize,
    pub hasta: usize,
    pub cantidad: usize,
    /// Clave para pedir la página siguiente / anterior; `None` si no hay.
    pub cursor_siguiente: Option<K>,
    pub cursor_anterior: Option<K>,
}

/// `items` debe venir ordenado por `clave`. Sin cursor devuelve la primera página.
pub fn paginar<T, K: Ord + Clone>(
    items: Vec<T>,
    clave: impl Fn(&T) -> K,
    cursor: Option<&K>,
    direccion: Direccion,
    limite: usize,
) -> Pagina<T, K> {
    let limite = limite.clamp(1, 500);
    let cantidad = items.len();
    let inicio = match (cursor, direccion) {
        (None, _) => 0,
        (Some(c), Direccion::Siguiente) => items.partition_point(|x| clave(x) <= *c),
        (Some(c), Direccion::Anterior) => items
            .partition_point(|x| clave(x) < *c)
            .saturating_sub(limite),
    };
    let fin = (inicio + limite).min(cantidad);
    let cursor_anterior = (inicio > 0).then(|| clave(&items[inicio]));
    let cursor_siguiente = (fin < cantidad && fin > inicio).then(|| clave(&items[fin - 1]));
    let elementos: Vec<T> = items.into_iter().skip(inicio).take(fin - inicio).collect();
    Pagina {
        desde: if elementos.is_empty() { 0 } else { inicio + 1 },
        hasta: fin,
        cantidad,
        elementos,
        cursor_siguiente,
        cursor_anterior,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(cursor: Option<i32>, d: Direccion) -> Pagina<i32, i32> {
        paginar((1..=25).collect(), |x| *x, cursor.as_ref(), d, 10)
    }

    #[test]
    fn avanza_y_retrocede() {
        let a = p(None, Direccion::Siguiente);
        assert_eq!(
            (a.desde, a.hasta, a.cursor_anterior, a.cursor_siguiente),
            (1, 10, None, Some(10))
        );
        let b = p(a.cursor_siguiente, Direccion::Siguiente);
        assert_eq!(b.elementos.first(), Some(&11));
        let c = p(b.cursor_siguiente, Direccion::Siguiente);
        assert_eq!((c.desde, c.hasta, c.cursor_siguiente), (21, 25, None));
        let d = p(c.cursor_anterior, Direccion::Anterior);
        assert_eq!(d.elementos, (11..=20).collect::<Vec<_>>());
        let e = p(d.cursor_anterior, Direccion::Anterior);
        assert_eq!((e.desde, e.cursor_anterior), (1, None));
    }

    #[test]
    fn cursor_de_un_elemento_borrado() {
        // La clave 10 ya no existe: la siguiente página empieza en 11 igual.
        let items: Vec<i32> = (1..=25).filter(|x| *x != 10).collect();
        let r = paginar(items, |x| *x, Some(&10), Direccion::Siguiente, 10);
        assert_eq!(r.elementos.first(), Some(&11));
    }

    #[test]
    fn vacia() {
        let r = paginar(Vec::<i32>::new(), |x| *x, None, Direccion::Siguiente, 10);
        assert_eq!(
            (r.desde, r.hasta, r.cantidad, r.cursor_siguiente),
            (0, 0, 0, None)
        );
    }
}
