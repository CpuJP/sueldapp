//! Máscara de los campos de dinero: miles con punto, decimales con coma (máximo 2).
//! Solo da formato al texto que se escribe; el valor lo interpreta `dominio::formato::leer_monto`.

/// "1234567,891" → "1.234.567,89". Un punto al final (teclado numérico) se toma como coma.
pub fn formatear_entrada(texto: &str) -> String {
    let mut t: String = texto
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
        .collect();
    // El punto de miles se pone solo: un "." recién escrito al final es una coma decimal.
    if t.ends_with('.') && !t.contains(',') {
        t.pop();
        t.push(',');
    }
    let t: String = t.chars().filter(|c| *c != '.').collect();
    let (entero, decimales) = match t.split_once(',') {
        Some((e, d)) => (
            e.to_string(),
            Some(d.replace(',', "").chars().take(2).collect::<String>()),
        ),
        None => (t, None),
    };
    let entero = entero.trim_start_matches('0');
    let entero = if entero.is_empty() {
        if decimales.is_some() || t_tiene_cero(texto) {
            "0".to_string()
        } else {
            String::new()
        }
    } else {
        entero.to_string()
    };
    let mut con_puntos = String::new();
    for (i, c) in entero.chars().enumerate() {
        if i > 0 && (entero.len() - i) % 3 == 0 {
            con_puntos.push('.');
        }
        con_puntos.push(c);
    }
    match decimales {
        Some(d) => format!("{con_puntos},{d}"),
        None => con_puntos,
    }
}

fn t_tiene_cero(texto: &str) -> bool {
    texto.chars().any(|c| c == '0') && !texto.chars().any(|c| c.is_ascii_digit() && c != '0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mascara() {
        for (a, b) in [
            ("1234567,891", "1.234.567,89"),
            ("0012,5", "12,5"),
            (",5", "0,5"),
            ("4200000", "4.200.000"),
            ("950.000,00", "950.000,00"),
            ("1,2,3", "1,23"),
            ("abc", ""),
            ("4.200.000.", "4.200.000,"),
            ("0", "0"),
        ] {
            assert_eq!(formatear_entrada(a), b, "{a}");
        }
    }
}
