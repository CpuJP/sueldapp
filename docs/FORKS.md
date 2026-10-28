# Guía para forks

Sueldapp tiene licencia MIT: puedes hacer un fork, adaptarlo y distribuirlo. Esta guía te ayuda a
hacerlo sin exponer datos personales y manteniéndote al día con el proyecto original.

## 1. Crear el fork

1. En GitHub, pulsa **Fork** en <https://github.com/CpuJP/sueldapp>.
2. Clónalo y añade el original como `upstream`:
   ```bash
   git clone git@github.com:<tu-usuario>/sueldapp.git
   cd sueldapp
   git remote add upstream https://github.com/CpuJP/sueldapp.git
   ```

## 2. Mantener tus datos fuera de git

El `.gitignore` ya excluye `.ods`, `.sqlite3`, `.csv`, `logs/`, `exports/`, `reglas.txt` y
`docs/privado/`. **No uses `git add -f` con esos archivos.** Antes de cada commit revisa
`git status`. Si por error subiste datos, no basta con borrarlos en un commit nuevo: hay que
reescribir el historial (por ejemplo con `git filter-repo`) y rotar lo expuesto.

Tus reglas personales (tarjetas, alias de deudas, categorías con nombres propios) van en
`reglas.txt` junto a tu base de datos. Usa `docs/ejemplos/reglas.ejemplo.txt` como punto de partida.

## 3. Personalizar tu versión

| Qué | Dónde |
|---|---|
| Nombre de la app y del ejecutable | `product-name` en `[package.metadata.packager]` y `name` en `crates/app/Cargo.toml` |
| Identificador único (obligatorio si distribuyes) | `identifier` en `[package.metadata.packager]` (p. ej. `io.github.<tu-usuario>.sueldapp`) |
| Editor, copyright, página web | `[package.metadata.packager]` en `crates/app/Cargo.toml` |
| Iconos | PNG/ICO en `crates/app/icons/` (mismos nombres) |
| Colores y estilo | global `Tema` en `crates/app/ui/tema.slint` |
| Formato de hoja distinto | `crates/ods/src/hoja.rs` (extractor) y `docs/analisis-hoja.md` |
| Moneda | `crates/dominio/src/formato.rs` |

Cambia el `identifier` para que tu versión no comparta la carpeta de datos ni choque con la original.

## 4. Traer cambios del original

```bash
git fetch upstream
git switch main
git merge upstream/main     # o: git rebase upstream/main
cargo test --workspace
```

## 5. Publicar tus propios instaladores

El flujo `.github/workflows/release.yml` funciona igual en tu fork:

1. Activa GitHub Actions en tu fork (pestaña **Actions**).
2. Sube la versión en `Cargo.toml` (`[workspace.package]`) y anota los cambios en `CHANGELOG.md`.
3. Crea y empuja una etiqueta: `git tag v0.2.0 && git push origin v0.2.0`.
4. Revisa el borrador de release que se crea con los instaladores y publícalo.

Más detalles en [`docs/RELEASES.md`](RELEASES.md).

## 6. Proponer tus cambios al original

Si tu cambio sirve a más gente, abre un Pull Request hacia `CpuJP/sueldapp:main` siguiendo
[`CONTRIBUTING.md`](../CONTRIBUTING.md).
