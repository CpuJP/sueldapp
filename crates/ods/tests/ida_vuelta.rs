//! Exportar a .ods y volver a importar en una base vacía conserva todo (SC-007).

use std::io::Cursor;
use std::str::FromStr;
use sueldapp_almacen::repo;
use sueldapp_dominio::{Decimal, Deuda, Gasto, Ingreso, Mes, Tarjeta};
use sueldapp_ods::escritura::{escribir_ods, tablas};
use sueldapp_ods::importar::importar_bytes;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn base_con_datos() -> sueldapp_almacen::Connection {
    let c = sueldapp_almacen::abrir_memoria().unwrap();
    let m = repo::guardar_mes(
        &c,
        &Mes {
            id: None,
            anio: 2026,
            mes: 10,
            cerrado: true,
        },
    )
    .unwrap()
    .id
    .unwrap();
    let deuda = repo::guardar_deuda(
        &c,
        &Deuda {
            id: None,
            nombre: "PC, \"gamer\" & <cosas>".into(),
            total: Some(d("4200000")),
            ajuste: d("0.68"),
            saldada: false,
            notas: "línea 1\nlínea 2".into(),
            origen: Some("Nómina!A8".into()),
        },
    )
    .unwrap()
    .id;
    let tarjeta = repo::guardar_tarjeta(
        &c,
        &Tarjeta {
            id: None,
            nombre: "Tarjeta Azul".into(),
            cupo: Some(d("5000000")),
            saldo: None,
            dia_corte: Some(15),
            dia_pago: None,
            origen: None,
        },
    )
    .unwrap()
    .id;
    for (i, (concepto, monto, estado)) in [
        ("Cuota 1 - Portátil", "350000.84", 1),
        ("Internet", "12500", 2),
        ("Donde Ana's", "31750.5", 5),
    ]
    .into_iter()
    .enumerate()
    {
        repo::guardar_gasto(
            &c,
            &Gasto {
                id: None,
                mes_id: m,
                concepto: concepto.into(),
                monto: d(monto),
                estado_id: estado,
                categoria_id: Some(3),
                deuda_id: if i == 0 { deuda } else { None },
                tarjeta_id: if i == 1 { tarjeta } else { None },
                desglose: Some("=12500+0".into()),
                nota: String::new(),
                fuera_de_bloque: i == 2,
                orden: i as i64,
                origen: None,
            },
        )
        .unwrap();
    }
    repo::guardar_ingreso(
        &c,
        &Ingreso {
            id: None,
            mes_id: m,
            fuente_id: 1,
            monto: d("3200000"),
            desglose: None,
            nota: String::new(),
            origen: None,
        },
    )
    .unwrap();
    c
}

#[test]
fn exportar_e_importar_conserva_todo() {
    let original = base_con_datos();
    let mut bytes = Cursor::new(Vec::new());
    escribir_ods(&tablas(&original).unwrap(), &mut bytes).unwrap();

    let mut nueva = sueldapp_almacen::abrir_memoria().unwrap();
    let inf = importar_bytes(&mut nueva, "export.ods", bytes.get_ref(), false).unwrap();
    assert!(inf.ok, "{}", inf.resumen());
    assert!(inf.conciliacion.comparados > 0);
    assert_eq!(
        repo::todos_gastos(&nueva).unwrap(),
        repo::todos_gastos(&original).unwrap()
    );
    assert_eq!(
        repo::todos_ingresos(&nueva).unwrap(),
        repo::todos_ingresos(&original).unwrap()
    );
    assert_eq!(
        repo::listar_deudas(&nueva).unwrap(),
        repo::listar_deudas(&original).unwrap()
    );
    assert_eq!(
        repo::listar_tarjetas(&nueva).unwrap(),
        repo::listar_tarjetas(&original).unwrap()
    );
    assert_eq!(
        repo::listar_estados(&nueva).unwrap(),
        repo::listar_estados(&original).unwrap()
    );
    assert_eq!(
        repo::listar_meses(&nueva).unwrap(),
        repo::listar_meses(&original).unwrap()
    );
}

#[test]
fn csv_con_montos_exactos() {
    let c = base_con_datos();
    let dir = std::env::temp_dir().join(format!("sueldapp-csv-{}", std::process::id()));
    let rutas = sueldapp_ods::csv::exportar_csv(&c, &dir).unwrap();
    let gastos = std::fs::read_to_string(dir.join("gastos.csv")).unwrap();
    assert!(gastos.contains(",350000.84,"));
    assert!(gastos.contains("Donde Ana's"));
    assert!(rutas.len() >= 8);
    let _ = std::fs::remove_dir_all(dir);
}
