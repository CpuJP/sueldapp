// Sin consola extra en Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capturas;
mod comandos;
mod demo;

use std::sync::Mutex;

pub struct Base(pub Mutex<sueldapp_almacen::Connection>);

fn main() {
    let ruta = sueldapp_almacen::ruta_por_defecto();
    let conexion = match sueldapp_almacen::abrir(&ruta) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("No se pudo abrir la base {}: {e}", ruta.display());
            std::process::exit(1);
        }
    };
    if std::env::var_os("SUELDAPP_DEMO").is_some() {
        if let Err(e) = demo::sembrar(&conexion) {
            eprintln!("No se pudieron crear los datos de demostración: {e}");
        }
    }
    let resultado = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Base(Mutex::new(conexion)))
        .invoke_handler(tauri::generate_handler![
            comandos::config,
            comandos::tablero,
            comandos::catalogos,
            comandos::listar_meses,
            comandos::guardar_mes,
            comandos::borrar_mes,
            comandos::restaurar_mes,
            comandos::resumen_mes,
            comandos::resumen_anio,
            comandos::buscar_gastos,
            comandos::guardar_gasto,
            comandos::borrar_gasto,
            comandos::restaurar_gasto,
            comandos::guardar_ingreso,
            comandos::borrar_ingreso,
            comandos::restaurar_ingreso,
            comandos::listar_deudas,
            comandos::guardar_deuda,
            comandos::borrar_deuda,
            comandos::restaurar_deuda,
            comandos::listar_tarjetas,
            comandos::guardar_tarjeta,
            comandos::borrar_tarjeta,
            comandos::restaurar_tarjeta,
            comandos::guardar_estado,
            comandos::borrar_estado,
            comandos::guardar_item,
            comandos::borrar_item,
            comandos::importar_ods,
            comandos::exportar_ods,
            comandos::exportar_csv,
            comandos::descargar_plantilla,
            comandos::historial_importaciones,
            capturas::capturar,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = resultado {
        eprintln!("Error al iniciar la ventana: {e}");
        std::process::exit(1);
    }
}
