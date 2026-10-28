//! Normalización de texto para búsquedas y para vincular cuotas por nombre.

use unicode_normalization::UnicodeNormalization;

/// Minúsculas y sin tildes: `"Crédito"` → `"credito"`.
pub fn normalizar(texto: &str) -> String {
    texto
        .nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

/// `true` si `texto` contiene `buscado`, sin distinguir mayúsculas ni tildes.
pub fn contiene(texto: &str, buscado: &str) -> bool {
    normalizar(texto).contains(&normalizar(buscado))
}

/// Concepto sin "cuota", "final", "mitad", números ni guiones, conservando mayúsculas y tildes.
/// `"Cuota - Tarjeta Crédito Azul"` → `"Tarjeta Crédito Azul"`.
pub fn etiqueta_concepto(concepto: &str) -> String {
    const IGNORAR: [&str; 5] = ["cuota", "cuotas", "coutas", "final", "mitad"];
    concepto
        .replace(['-', '–', '—'], " ")
        .split_whitespace()
        .filter(|p| {
            !IGNORAR.contains(&normalizar(p).as_str()) && !p.chars().all(|c| c.is_ascii_digit())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Raíz de un concepto de cuota, para agrupar: `"Consola XL – Cuota 6"` → `"consola xl"`;
/// `"Cuota 4 - PC"` → `"pc"`.
pub fn raiz_concepto(concepto: &str) -> String {
    normalizar(&etiqueta_concepto(concepto))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normaliza() {
        assert_eq!(normalizar("Crédito AÉREO"), "credito aereo");
        assert!(contiene("Cuota - Tarjeta Crédito Azul", "azul"));
        assert!(contiene("Audifonos – Cuota 5", "AUDÍFONOS"));
    }

    #[test]
    fn raices() {
        assert_eq!(raiz_concepto("Consola XL – Cuota 6"), "consola xl");
        assert_eq!(raiz_concepto("Cuota 4 - PC"), "pc");
        assert_eq!(raiz_concepto("Cel Ana- Cuota 2"), "cel ana");
        assert_eq!(raiz_concepto("Cuota 2 - Monitor - Mitad"), "monitor");
        assert_eq!(raiz_concepto("Viaje Playa - Cuota 10"), "viaje playa");
        assert_eq!(raiz_concepto("TV5 – Cuota 16"), "tv5");
        assert_eq!(
            etiqueta_concepto("Cuota - Tarjeta Crédito Azul"),
            "Tarjeta Crédito Azul"
        );
    }
}
