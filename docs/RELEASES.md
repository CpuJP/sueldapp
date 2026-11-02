# Cómo se publican las versiones

## Plataformas

| Sistema | Arquitectura | Instaladores |
|---|---|---|
| Linux | x64 (amd64) | `.deb`, `.rpm`, `.AppImage` |
| Linux | ARM64 (aarch64) | `.deb`, `.rpm` |
| Windows | x64 | `-setup.exe` (NSIS), `.msi` |
| Windows | ARM64 | `-setup.exe` (NSIS) |

Cada instalador se construye de forma nativa en su arquitectura (runners de GitHub `ubuntu-22.04`,
`ubuntu-22.04-arm`, `windows-latest` y `windows-11-arm`). La release incluye `SHA256SUMS.txt`.

## Versionado

[SemVer](https://semver.org/lang/es/): `MAJOR.MINOR.PATCH`. Las preversiones llevan sufijo
(`v0.1.0-alpha.1`, `v0.2.0-beta.1`) y se publican como *pre-release*. Las etiquetas sin sufijo
(`v0.2.0`) se publican como versión estable. La versión de `Cargo.toml` (`[workspace.package]`) es la
parte numérica (`0.2.0`); el instalador MSI de Windows no admite sufijos.

## Pasos

1. Actualiza la versión en `Cargo.toml` (`[workspace.package]`) y en las dependencias internas
   (`sueldapp-* = { version = "…", path = … }` de `crates/*/Cargo.toml`).
2. Mueve lo de "Sin publicar" de `CHANGELOG.md` a una sección nueva con la versión y la fecha.
3. Añade a `crates/almacen/tests/versiones.rs` una prueba `v0_4_0_…` con lo específico de la versión.
   Corre `cargo test --workspace` con tu `.ods`: la conciliación real no se ejecuta en GitHub.
4. Commit: `chore: versión 0.4.0`.
5. Etiqueta y empuja:
   ```bash
   git tag -a v0.4.0 -m "Sueldapp 0.4.0"
   git push origin main v0.4.0
   ```
6. El flujo **Release** hace todo lo demás, en orden:
   - Exige que la etiqueta coincida con la versión, la sección del CHANGELOG y la prueba de la versión.
   - Pasa formato, lints y pruebas en las cuatro plataformas, y la prueba de humo de la app en Linux.
   - Construye los instaladores y los sube al borrador con las notas del CHANGELOG.
   - Publica la release.

Si algo falla, la release se queda en **borrador** y no llega a nadie. Nadie prueba los instaladores
a mano antes de publicar: cada instalador se construye, pero solo se ejecuta la app de Linux x64.

Para reconstruir una etiqueta existente: Actions → Release → *Run workflow* → indica la etiqueta.

## Herramientas

`cargo-packager` genera `.deb`, `.AppImage`, NSIS y MSI con la configuración de
`[package.metadata.packager]` en `crates/app/Cargo.toml`; `cargo-generate-rpm` genera el `.rpm`
(`[package.metadata.generate-rpm]`). En local:

```bash
cargo install cargo-packager cargo-generate-rpm
NO_STRIP=true cargo packager --release -p sueldapp -f deb,appimage   # salida en target/paquetes/
cargo generate-rpm -p crates/app                                     # salida en target/generate-rpm/
```

## Firma de código

Los instaladores no están firmados todavía. Windows SmartScreen puede mostrar un aviso la primera
vez ("Más información" → "Ejecutar de todas formas"). Verifica la descarga con `SHA256SUMS.txt`.
