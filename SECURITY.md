# Política de seguridad

## Versiones con soporte

Solo la última versión estable publicada recibe correcciones de seguridad. Las preversiones
(`-alpha`, `-beta`) no tienen soporte.

## Cómo reportar una vulnerabilidad

**No abras un issue público.** Usa el formulario privado de GitHub:
[Reportar una vulnerabilidad](https://github.com/CpuJP/sueldapp/security/advisories/new).

Incluye la versión, el sistema y los pasos para reproducirla, **sin datos financieros reales**.
Recibirás respuesta en un plazo de 7 días.

## Modelo de seguridad

- Sueldapp no hace conexiones de red en tiempo de ejecución. La interfaz es nativa (Slint), sin
  motor web, y no carga recursos externos.
- Los datos se guardan en una base SQLite local, sin cifrar, en la carpeta de datos del usuario.
  La protección del archivo depende de los permisos del sistema y del cifrado del disco.
- El importador solo lee los archivos `.ods`; nunca los modifica.
- Los instaladores no descargan nada al instalarse.
