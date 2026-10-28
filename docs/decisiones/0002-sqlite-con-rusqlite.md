# ADR 0002 — SQLite con rusqlite y migraciones propias

- Estado: aceptada

## Decisión

Usar `rusqlite` con la feature `bundled`. Las migraciones son archivos SQL embebidos y se aplican
según `PRAGMA user_version`. La base usa modo WAL y claves foráneas.

## Alternativas

- `sqlx`: su modelo async y la comprobación en compilación no aportan nada a un único usuario
  local.
- `refinery`: es un framework para lo que aquí es una lista ordenada de archivos.

## Consecuencias

Nunca se edita una migración publicada; los cambios de esquema van en una migración nueva.
