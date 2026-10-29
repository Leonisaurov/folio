# AGENTS.md — Folio

## Empieza aquí

1. Lee `README.md`, luego `PLAN.md`, luego `docs/09-hoja-de-ruta.md` (etapa **E0**).
2. Para ejecutar el día 1: `docs/13-primeros-pasos.md`.
3. Para tests y automatización: `docs/15-automatizacion.md`.

**Comprueba el estado real del repo.** Si no existe `Cargo.toml`, está en fase de plan/E0: no inventes la app ni crees un `Cargo.toml` falso para que los scripts “pasen”. El siguiente trabajo real es **E0: repositorio, CI, ventana GTK, automatización aislada y benchmark base**, no una feature de notas. Si `Cargo.toml` ya existe, respeta la etapa y las puertas de `docs/09`.

La fuente de verdad es: `docs/09` para el orden, `docs/10` para decisiones, `docs/03` para datos, `docs/05` para configuración, `docs/06` para plugins y `docs/15` para automatización. Si la prosa contradice un script o configuración ejecutable, gana el script y se corrige la prosa.

## Reglas duras

- **No adelantes etapas.** No se empieza E1 sin pasar la puerta de E0; no se implementan plugins antes de E10.
- **No metas en el MVP**: sync, colaboración, DOCX, OCR, IA, marketplace, móvil. Están en `docs/12`.
- **Cifrado, web viva, PDF, `quickbar`/`stats` y `.deb` son 1.0**, no MVP. La decisión Q-26 está en `docs/11`.
- **Nunca ejecutes tests GTK en el `DISPLAY` del usuario (`:0`).** Usa `scripts/automation/ui.sh` o `with-x11.sh`. La regla y el aislamiento están en `docs/15`.
- **Nunca uses `~/Notas` ni `~/.config/folio` en tests.** `with-x11.sh` crea `HOME`/XDG temporales; respétalos.
- **No declares que algo funciona sin ejecutar la suite.** Si no hay código, di “plan validado”, no “app probada”.
- **Fuente de verdad de datos**: archivos JSON versionados + assets; SQLite es solo índice reconstruible. Escritura atómica y migraciones son no negociables (`docs/03`).
- **Dependencias entre crates**: `folio-core`, `folio-store`, `folio-daily`, `folio-layout` y `folio-plugins` no dependen de GTK. `folio-ui` no habla con SQLite directamente (`docs/02`).
- **Configuración**: una sola definición de esquema; todo cambio debe aparecer en GUI, TOML, CLI y `docs/05`, con prueba de paridad.
- **Plugins**: sin autoridad ambiental; permisos explícitos, límites de CPU/RAM y UI declarativa (`docs/06`).
- **Cambiar alcance o una decisión** exige actualizar `docs/10`, `docs/09` y `docs/14`, y ejecutar `./scripts/automation/plan.sh`.

## Comandos

```bash
# Suite completa: plan + calidad + dominio + UI en Xvfb privado
./scripts/automation/all.sh

# Iteración rápida sin UI
./scripts/automation/all.sh --fast

# Pasos sueltos
./scripts/automation/plan.sh
./scripts/automation/quality.sh
./scripts/automation/unit.sh
./scripts/automation/ui.sh --test smoke

# Comprobar que el display es privado (debe ser distinto de :0)
./scripts/automation/with-x11.sh -- sh -c 'echo "DISPLAY=$DISPLAY"'

# Dejar una suite larga sin que la máquina se suspenda
./scripts/automation/keep-awake.sh -- ./scripts/automation/all.sh
./scripts/automation/keep-awake.sh --lid -- ./scripts/automation/watch.sh
./scripts/automation/nightly.sh

# Atajos si está instalado `just`
just check
just check-fast
just test-ui
```

- Sin `Cargo.toml`, `quality.sh`/`unit.sh`/`ui.sh` se omiten a propósito; `plan.sh` sí se ejecuta. No lo interpretes como “tests de la app pasando”.
- Los tests de UI corren con `--test-threads=1`; no los paralelices compartiendo display ni `XDG_RUNTIME_DIR`.
- Artefactos en `.artifacts/`; `FOLIO_TEST_KEEP=1` conserva el entorno temporal para depurar.
- `keep-awake.sh` inhibe `sleep`, no `idle`: la pantalla puede apagarse. `--lid` solo con corriente y ventilación.

## Mapa mínimo

| Necesitas… | Archivo |
|---|---|
| Alcance, glosario, aceptación | `docs/01-vision-alcance.md` |
| Crates, editor, canvas, storage | `docs/02-arquitectura.md` |
| JSON, orden, SQLite, backups, cifrado | `docs/03-modelo-datos.md` |
| UI, DnD, atajos, accesibilidad | `docs/04-experiencia-ui.md` |
| TOML, GUI, CLI, claves | `docs/05-configuracion.md` |
| WASM/WIT, permisos, ejemplos | `docs/06-plugins.md` |
| Amenazas, sandbox, rendimiento | `docs/07-seguridad-rendimiento.md` |
| Pruebas, CI, release | `docs/08-calidad-pruebas.md` |
| Etapas y puertas | `docs/09-hoja-de-ruta.md` |
| ADR y decisiones | `docs/10-decisiones.md` |
| Dudas con defecto | `docs/11-preguntas-abiertas.md` |
| Xvfb, artefactos, keep-awake | `docs/15-automatizacion.md` |

## Definición de terminado

- `./scripts/automation/all.sh` pasa (o se explica exactamente por qué se omitió código).
- La documentación afectada está actualizada; si cambió una decisión, hay ADR.
- No hay features fuera de alcance ni archivos de usuario tocados por tests.
- El resultado se demuestra con salida de comandos o rutas de archivos, no con afirmaciones.
