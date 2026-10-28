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
(`v0.1.0-alpha.1`, `v0.2.0-beta.1`) y se publican como *pre-release*. La versión de
`crates/app/tauri.conf.json` y de `Cargo.toml` es la parte numérica (`0.1.0`); el instalador MSI de
Windows no admite sufijos.

## Pasos

1. Actualiza la versión en `Cargo.toml` (`[workspace.package]`) y en `crates/app/tauri.conf.json`.
2. Mueve lo de "Sin publicar" de `CHANGELOG.md` a una sección nueva con la versión y la fecha.
3. Commit: `chore: versión 0.2.0`.
4. Etiqueta y empuja:
   ```bash
   git tag -a v0.2.0 -m "Sueldapp 0.2.0"
   git push origin main v0.2.0
   ```
5. El flujo **Release** comprueba que la etiqueta coincide con la versión, compila en las cuatro
   plataformas y sube los instaladores a un **borrador** de release con las notas del CHANGELOG.
6. Revisa el borrador, prueba los instaladores y pulsa **Publish release**.

Para reconstruir una etiqueta existente: Actions → Release → *Run workflow* → indica la etiqueta.

## Firma de código

Los instaladores no están firmados todavía. Windows SmartScreen puede mostrar un aviso la primera
vez ("Más información" → "Ejecutar de todas formas"). Verifica la descarga con `SHA256SUMS.txt`.
