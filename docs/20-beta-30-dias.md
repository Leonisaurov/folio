# 20 — Beta de 30 días

## 1. Objetivo

Validar Folio con uso real durante 30 días sin telemetría, sin pérdida de datos y
con un canal de rollback probado. La beta empieza al publicar el canal
`dist/beta/<versión>` y termina con el checklist de salida de este documento.

## 2. Canal

```bash
./scripts/beta.sh
```

Genera:

```text
dist/beta/<versión>/
├── CHANNEL.json          # channel=beta, telemetry=false, schema=1
├── CHECKLIST.md          # este documento
├── REPORT.txt            # salida de beta-check
├── folio_<versión>_<arch>.deb
├── folio-<versión>-linux-x86_64.tar.gz
└── SHA256SUMS
```

- No hay servidor de actualizaciones ni telemetría. El tester instala el `.deb` o
  extrae el tarball y reporta manualmente.
- Verificación antes de publicar: `./scripts/beta-check.sh`.
- Distribución: issue privado, correo o carpeta compartida; nunca datos reales en
  el repositorio.

## 3. Antes del día 1

- [ ] `./scripts/automation/all.sh` verde.
- [ ] `./scripts/security.sh` verde.
- [ ] `./scripts/a11y.sh` verde.
- [ ] `./scripts/build-deb.sh` verificado.
- [ ] `./scripts/beta.sh` genera artefactos y `SHA256SUMS`.
- [ ] `docs/21-release-1.0.md` revisado.
- [ ] 5–10 testers aceptados; cada uno con backup inicial.
- [ ] Canal de feedback y responsable de triage definidos.
- [ ] Confirmar que no se recoge telemetría ni se activa red por defecto.

## 4. Rutina diaria (30 días)

| Día | Acción |
|---|---|
| 1 | Instalar desde `.deb`, crear bóveda, escribir 3 notas, comprobar autosave. |
| 2 | Probar libros diarios y cambio de hora de corte. |
| 3 | DnD entre libros, spring-load y «Mover a…». |
| 4 | Tabla, imagen, audio/vídeo y tarjeta web offline. |
| 5 | Exportar MD/HTML/PDF y reimportar el MD. |
| 6 | Buscar con y sin acentos; abrir resultado. |
| 7 | Backup, verificar y restaurar en una copia; `doctor`. |
| 8 | Cifrar bóveda, bloquear, desbloquear y recuperar con frase. |
| 9 | Probar plugin progress/chart/diagram y revocar permisos. |
| 10 | Revisión semanal: P0/P1, rendimiento, RAM, discos. |
| 11–17 | Uso normal diario; registrar cualquier pérdida o crash. |
| 18 | Migración simulada: vault de fixture, campos desconocidos, restore. |
| 19 | Accesibilidad: teclado completo, escala 200 %, lector de pantalla. |
| 20 | Revisión semanal y decisión de parche beta. |
| 21–27 | Uso normal; probar actualización desde `.deb` anterior. |
| 28 | Rollback drill: reinstalar beta anterior y restaurar backup. |
| 29 | Cierre de incidencias P1; revisar documentación y changelog. |
| 30 | Checklist de salida; congelar candidato 1.0. |

Cada día se registra: versión, acción, resultado, datos afectados (nunca adjuntar
la bóveda real), log sin secretos y captura si es visual.

## 5. Triage P0/P1

| Severidad | Definición | Respuesta | Acción |
|---|---|---|---|
| **P0** | Pérdida/corrupción de datos, ejecución de código no confiable, crash loop, fuga de clave o cifrado roto | Inmediata, mismo día | Detener beta, preservar evidencia, parche o rollback |
| **P1** | Bloquea una tarea central (abrir/guardar/buscar/exportar/desbloquear) sin workaround | ≤ 72 h | Parche beta y re-test |
| **P2** | Función secundaria degradada, workaround disponible | ≤ 7 días | Entra en 1.0 o backlog |
| **P3** | Cosmético/mejora | Sin plazo | Backlog |

Un P0 de datos se convierte en prueba de regresión antes de corregir. Nunca se
publica un parche que borre o reescriba datos sin backup previo.

## 6. Migraciones

- `schema = 1` es la versión actual. Un `schema` mayor se rechaza y muestra
  «Actualiza Folio»; no se abre ni se reescribe.
- Los campos desconocidos se preservan (probado en `beta-check.sh`).
- Antes de una migración: backup verificado, copia de la bóveda, `doctor`, y
  restauración de prueba en una ruta nueva.
- Una migración solo se publica con fixture antiguo, test de roundtrip y plan de
  rollback.

## 7. Rollback

1. Detener la app y conservar la bóveda.
2. Instalar la versión beta anterior:
   `sudo dpkg -i folio_<versión-anterior>_amd64.deb`.
3. Si la bóveda se migró, restaurar el backup **en una copia**:
   `folio --vault <copia> backup restore <backup> <destino>`.
4. Ejecutar `folio doctor --rebuild-index` y verificar notas/assets.
5. Confirmar checksums y registrar el incidente.

Nunca se restaura in-place sin confirmación. Si el fallo es de clave/cifrado, se
detiene la beta y se conserva una copia cifrada intacta.

## 8. Salida de la beta

- [ ] 30 días de uso, ≥ 5 testers, sin P0 abiertos.
- [ ] 0 corrupciones; restore drill y rollback drill documentados.
- [ ] Migración/campos desconocidos verificados.
- [ ] Auditoría manual con Orca/Wayland completada.
- [ ] Flatpak/AUR probados en VM limpia (o Plan B documentado).
- [ ] Rendimiento dentro de presupuesto o excepción aprobada.
- [ ] Documentación, changelog y checksums del candidato 1.0.
- [ ] Decisión sobre runtime WASM de terceros (1.0 o 1.1) registrada en ADR.
