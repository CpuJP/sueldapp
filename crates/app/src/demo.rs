//! Datos ficticios para capturas de pantalla y la guía de usuario (`SUELDAPP_DEMO=1`).
//! Solo se crean si la base está vacía. Nunca usar con la base real.

use std::str::FromStr;
use sueldapp_almacen::repo::{self, Catalogo};
use sueldapp_almacen::{Connection, Resultado};
use sueldapp_dominio::{Decimal, Deuda, Gasto, Ingreso, Mes, Tarjeta};

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_default()
}

/// Concepto, monto, estado, categoría, deuda, tarjeta.
type FilaDemo<'a> = (String, &'a str, i64, Option<i64>, Option<i64>, Option<i64>);

pub fn sembrar(c: &Connection) -> Resultado<()> {
    if !repo::listar_meses(c)?.is_empty() {
        return Ok(());
    }
    let pagado = repo::id_estado(c, "Pagado")?;
    let pendiente = repo::id_estado(c, "Pendiente")?;
    let aplazado = repo::id_estado(c, "Aplazado")?;
    let servicios = repo::id_item(c, Catalogo::Categorias, "Servicios")?;
    let tarjetas = repo::id_item(c, Catalogo::Categorias, "Tarjetas")?;
    let personal = repo::id_item(c, Catalogo::Categorias, "Personal")?;
    let sueldo = repo::id_item(c, Catalogo::Fuentes, "Sueldo")?;
    let prima = repo::id_item(c, Catalogo::Fuentes, "Prima")?;

    let deuda = |nombre: &str, total: &str| {
        repo::guardar_deuda(
            c,
            &Deuda {
                id: None,
                nombre: nombre.into(),
                total: Some(d(total)),
                ajuste: Decimal::ZERO,
                saldada: false,
                notas: String::new(),
                origen: None,
            },
        )
        .map(|x| x.id)
    };
    let portatil = deuda("Portátil", "4200000")?;
    let bici = deuda("Bicicleta", "1800000")?;
    let tarjeta = repo::guardar_tarjeta(
        c,
        &Tarjeta {
            id: None,
            nombre: "Tarjeta Azul".into(),
            cupo: Some(d("6000000")),
            saldo: Some(d("2350000")),
            dia_corte: Some(15),
            dia_pago: Some(30),
            origen: None,
        },
    )?
    .id;

    for (i, mes) in (7..=12).enumerate() {
        let m = repo::guardar_mes(
            c,
            &Mes {
                id: None,
                anio: 2026,
                mes,
                cerrado: mes < 10,
            },
        )?
        .id
        .unwrap_or_default();
        let estado = if mes < 10 { pagado } else { pendiente };
        let ingreso = |fuente, monto: &str| {
            repo::guardar_ingreso(
                c,
                &Ingreso {
                    id: None,
                    mes_id: m,
                    fuente_id: fuente,
                    monto: d(monto),
                    desglose: None,
                    nota: String::new(),
                    origen: None,
                },
            )
        };
        ingreso(sueldo, "3200000")?;
        if mes == 12 {
            ingreso(prima, "1500000")?;
        }
        let n = i as i64 + 1;
        let gastos: Vec<FilaDemo> = vec![
            (
                "Arriendo".into(),
                "950000",
                estado,
                Some(personal),
                None,
                None,
            ),
            (
                "Internet y móvil".into(),
                "89900",
                estado,
                Some(servicios),
                None,
                None,
            ),
            (
                "Mercado".into(),
                "520000",
                estado,
                Some(personal),
                None,
                None,
            ),
            (
                format!("Portátil – Cuota {n}"),
                "350000",
                estado,
                None,
                portatil,
                None,
            ),
            (
                format!("Bicicleta – Cuota {n}"),
                "150000",
                if mes == 10 { aplazado } else { estado },
                None,
                bici,
                None,
            ),
            (
                "Pago mínimo Tarjeta Azul".into(),
                "410000",
                estado,
                Some(tarjetas),
                None,
                tarjeta,
            ),
        ];
        for (orden, (concepto, monto, estado_id, categoria_id, deuda_id, tarjeta_id)) in
            gastos.into_iter().enumerate()
        {
            repo::guardar_gasto(
                c,
                &Gasto {
                    id: None,
                    mes_id: m,
                    concepto,
                    monto: d(monto),
                    estado_id,
                    categoria_id,
                    deuda_id,
                    tarjeta_id,
                    desglose: None,
                    nota: String::new(),
                    fuera_de_bloque: false,
                    orden: orden as i64,
                    origen: None,
                },
            )?;
        }
    }
    Ok(())
}
