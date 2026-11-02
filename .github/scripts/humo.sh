#!/usr/bin/env bash
# Abre la app con datos de demostración y recorre pantallas, temas y gestos de ratón.
# Falla si la prueba de humo no termina bien. Uso: humo.sh <ruta del binario>
set -u
dir=$(mktemp -d)
SUELDAPP_DB="$dir/demo.sqlite3" SUELDAPP_DEMO=1 SUELDAPP_CAPTURAS="$dir/capturas" \
  timeout 900 xvfb-run -a -s "-screen 0 1600x2400x24" "$1"
if ls "$dir"/capturas/prueba-*fallo* >/dev/null 2>&1; then
  for f in "$dir"/capturas/prueba-*fallo*; do echo "::error::$(basename "$f"): $(cat "$f")"; done
  exit 1
fi
if [ ! -f "$dir/capturas/prueba-ok.txt" ]; then
  echo "::error::La prueba de humo no llegó al final"
  exit 1
fi
echo "Prueba de humo correcta"
