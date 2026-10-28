//! Capturas de pantalla para revisar el diseño y la guía de usuario (solo desarrollo).
//! Se compila con `--features capturas` y se activa con `SUELDAPP_CAPTURAS=<carpeta>`.
//! Usa la API de WebKit porque las herramientas de captura de X no leen la superficie GL.

#[tauri::command]
pub async fn capturar(ventana: tauri::WebviewWindow, nombre: String) -> Result<String, String> {
    let carpeta = std::env::var("SUELDAPP_CAPTURAS")
        .map_err(|_| "Define SUELDAPP_CAPTURAS para capturar.".to_string())?;
    let limpio: String = nombre
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let ruta = std::path::Path::new(&carpeta).join(format!("{limpio}.png"));
    guardar(ventana, ruta).await
}

#[cfg(all(feature = "capturas", target_os = "linux"))]
async fn guardar(
    ventana: tauri::WebviewWindow,
    ruta: std::path::PathBuf,
) -> Result<String, String> {
    use webkit2gtk::{SnapshotOptions, SnapshotRegion, WebViewExt};
    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    ventana
        .with_webview(move |w| {
            let tx = tx.clone();
            w.inner().snapshot(
                SnapshotRegion::Visible,
                SnapshotOptions::NONE,
                None::<&webkit2gtk::gio::Cancellable>,
                move |r| {
                    let res = r
                        .map_err(|e| e.to_string())
                        .and_then(|s| {
                            cairo::ImageSurface::try_from(s)
                                .map_err(|_| "superficie no válida".to_string())
                        })
                        .and_then(|img| {
                            let mut f = std::fs::File::create(&ruta).map_err(|e| e.to_string())?;
                            img.write_to_png(&mut f).map_err(|e| e.to_string())?;
                            Ok(ruta.display().to_string())
                        });
                    let _ = tx.send(res);
                },
            );
        })
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rx.recv().map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(not(all(feature = "capturas", target_os = "linux")))]
async fn guardar(_v: tauri::WebviewWindow, _r: std::path::PathBuf) -> Result<String, String> {
    Err("Esta versión no incluye capturas (compila con --features capturas).".into())
}
