//! Pruebas de cada versión publicada: abrir sin perder datos una base de la versión anterior.
//! El flujo Release exige una función `vX_Y_Z` para la versión que se publica.

use sueldapp_almacen::{abrir, repo};

/// Una base como la que dejó la 0.3.0: solo la primera migración, con datos inventados.
#[test]
fn v0_4_0_abre_una_base_de_la_0_3_0() {
    let dir = std::env::temp_dir().join(format!("sueldapp-versiones-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join("base.sqlite3");
    let _ = std::fs::remove_file(&ruta);
    {
        let c = rusqlite::Connection::open(&ruta).unwrap();
        c.execute_batch(include_str!("../migraciones/0001_inicial.sql"))
            .unwrap();
        c.execute_batch(
            "PRAGMA user_version = 1;
             INSERT INTO meses (id, anio, mes, cerrado) VALUES (1, 2026, 10, 1);
             INSERT INTO gastos (id, mes_id, concepto, monto, estado_id, origen)
             SELECT 1, 1, 'Internet', '12500.50', id, 'Hoja!A1' FROM estados LIMIT 1;",
        )
        .unwrap();
    }

    let c = abrir(&ruta).unwrap();
    let g = repo::gasto(&c, 1).unwrap();
    assert_eq!(g.concepto, "Internet");
    assert_eq!(g.monto.to_string(), "12500.50");
    assert_eq!(g.origen.as_deref(), Some("Hoja!A1"));
    assert_eq!(g.fecha_pago, None);
    assert!(repo::mes(&c, 1).unwrap().cerrado);

    // Se puede guardar una fecha, y volver a abrir no repite la migración.
    let mut g = g;
    g.fecha_pago = Some("2026-11-23".into());
    repo::guardar_gasto(&c, &g).unwrap();
    drop(c);
    let c = abrir(&ruta).unwrap();
    assert_eq!(
        repo::gasto(&c, 1).unwrap().fecha_pago.as_deref(),
        Some("2026-11-23")
    );
    drop(c);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Una base de la 0.4.0 ya tiene todas las migraciones: abrirla no cambia nada.
#[test]
fn v0_4_1_abre_una_base_de_la_0_4_0() {
    let dir = std::env::temp_dir().join(format!("sueldapp-v041-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join("base.sqlite3");
    let _ = std::fs::remove_file(&ruta);
    {
        let c = rusqlite::Connection::open(&ruta).unwrap();
        c.execute_batch(include_str!("../migraciones/0001_inicial.sql"))
            .unwrap();
        c.execute_batch(include_str!("../migraciones/0002_fecha_pago.sql"))
            .unwrap();
        c.execute_batch(
            "PRAGMA user_version = 2;
             INSERT INTO meses (id, anio, mes, cerrado) VALUES (1, 2026, 10, 0);
             INSERT INTO gastos (id, mes_id, concepto, monto, estado_id, fecha_pago)
             SELECT 1, 1, 'Cuota', '100', id, '2026-11-23' FROM estados LIMIT 1;",
        )
        .unwrap();
    }

    let c = abrir(&ruta).unwrap();
    assert_eq!(
        repo::gasto(&c, 1).unwrap().fecha_pago.as_deref(),
        Some("2026-11-23")
    );
    drop(c);
    let _ = std::fs::remove_dir_all(&dir);
}
