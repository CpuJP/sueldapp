@AGENTS.md

## Específico de Claude Code

Reglas por tema: @.claude/rules/rust.md @.claude/rules/ui.md @.claude/rules/datos-financieros.md
@.claude/rules/git.md

Subagentes (`.claude/agents/`):
- `analista-datos`: leer el `.ods`, importador, conciliación.
- `arquitecto-rust`: dominio, persistencia, errores, rendimiento.
- `disenador-ui`: interfaz, estilo vidrio, accesibilidad, revisión de capturas.
- `tester`: pruebas unitarias, de integración y de conciliación.
- `revisor`: revisión de código y de la constitución (solo lectura), antes de cerrar una fase.

Puntos de parada (esperar al usuario): tras `/speckit-clarify` si quedan preguntas, tras
`/speckit-plan`, tras `/speckit-analyze`, y antes de cualquier acción destructiva o irreversible.
