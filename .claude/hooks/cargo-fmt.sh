#!/usr/bin/env bash
# Formatea el archivo .rs recién editado. Lee el JSON del hook por stdin.
archivo=$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("tool_input",{}).get("file_path",""))' 2>/dev/null)
case "$archivo" in
  *.rs) rustfmt --edition 2021 "$archivo" >/dev/null 2>&1 || true ;;
esac
exit 0
