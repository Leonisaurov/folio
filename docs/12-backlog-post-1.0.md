# 12 — Backlog posterior a 1.0

Ordenado por valor/riesgo. No es una promesa de fechas; es el orden en que se reevaluará después de estabilizar 1.0.

## 1. Prioridad alta (1.1–1.2)

| ID | Función | Por qué | Depende de |
|---|---|---|---|
| B-01 | Sincronización local/WebDAV/S3 con CRDT | Continuidad entre máquinas sin nube propietaria | Modelo estable, IDs, assets |
| B-02 | Etiquetas y filtros guardados | Organización transversal | Búsqueda |
| B-03 | Backlinks y referencias entre notas | Conectar ideas | Modelo de enlaces |
| B-04 | Plantillas de libro y de nota completas | Reutilización | Editor |
| B-05 | Historial de versiones en UI | Confianza y recuperación | E11 |
| B-06 | Importadores Obsidian/Notion/Evernote | Migración | Parser MD/HTML |
| B-07 | Grabación de audio y transcripción local opcional | Captura rápida | GStreamer/Whisper local |
| B-08 | Calendario/heatmap de actividad | Valor de los libros diarios | Índice |
| B-09 | Atajos configurables en GUI | Accesibilidad/personalización | Config |
| B-10 | Búsqueda en PDFs adjuntos (sin OCR) | Utilidad | Extracción de texto |

## 2. Prioridad media (1.3–1.5)

| ID | Función | Por qué | Depende de |
|---|---|---|---|
| B-11 | Marketplace/curado de plugins | Descubrimiento | Firma, revisión, servidor |
| B-12 | SDK TypeScript/JS | Más autores | WIT estable |
| B-13 | Temas por tokens y CSS limitado | Personalización | Seguridad |
| B-14 | Vistas guardadas y filtros complejos | Flujos personales | Búsqueda |
| B-15 | Ventanas múltiples/pestañas | Comparar notas | Estado de UI |
| B-16 | Grupos de bloques y conectores | Diagramas ricos | Canvas |
| B-17 | Rotación y capas en canvas | Libertad visual | Canvas |
| B-18 | Columnas/callouts/toggles | Documentos ricos | Modelo de bloques |
| B-19 | Importación/exportación DOCX | Intercambio ofimático | Render |
| B-20 | Impresión directa | Flujo físico | PDF |

## 3. Prioridad baja / exploratoria

| ID | Función | Nota |
|---|---|---|
| B-21 | OCR de imágenes | Local, opt-in |
| B-22 | Reconocimiento de escritura | Stylus, local |
| B-23 | IA local para resumir/buscar | Plugin, sin nube por defecto |
| B-24 | Colaboración en tiempo real | Servidor/CRDT, producto distinto |
| B-25 | Aplicación móvil | Fuera de foco Linux |
| B-26 | Versión web | Fuera de foco |
| B-27 | Correo/calendario integrados | Fuera de foco |
| B-28 | Grabación de pantalla | Plugin |
| B-29 | Publicación de notas como web | Export HTML |
| B-30 | Firma/verificación de notas | Casos legales |

## 4. Criterios para aceptar una función nueva

Antes de entrar al backlog activo:

1. ¿Resuelve un dolor real de uso diario?
2. ¿Se puede hacer como plugin sin tocar el core?
3. ¿Respeta los presupuestos de rendimiento?
4. ¿Tiene un modelo de datos claro y migración?
5. ¿Se puede probar y recuperar?
6. ¿Añade una dependencia de seguridad crítica?

Si la respuesta 2 es sí, va primero como plugin. Si la 6 es sí, requiere ADR y revisión de seguridad.

## 5. Sincronización: diseño preparado

Aunque no se implemente, el modelo ya cumple:

- IDs UUIDv7 estables y únicos por cliente.
- Operaciones de dominio serializables.
- Assets por hash BLAKE3 (contenido direccionado).
- Orden por claves fraccionales (fácil de reconciliar).
- Metadatos de versión y `updated_at`.
- Sin rutas absolutas en el contenido.
- Bloques desconocidos preservados.

Cuando se decida el backend (semana 14), se evaluará:

| Opción | Ventaja | Riesgo |
|---|---|---|
| WebDAV | Compatible con Nextcloud, fácil | Conflictos, latencia |
| S3 compatible | Barato, ubicuo | Credenciales, consistencia |
| Carpeta sincronizada (Syncthing/Dropbox) | Cero servidor propio | Conflictos de archivos |
| Servidor propio | Control, tiempo real | Mantenimiento, cuentas |

La sync de Folio no debería depender de un servicio propietario; el usuario elige el almacenamiento.

## 6. Lo que no se hará

- Rastrear al usuario.
- Bloquear la exportación.
- Requerir cuenta para usar la app.
- Ejecutar plugins sin sandbox.
- Adoptar un formato binario opaco como única copia.
- Prometer compatibilidad perfecta con DOCX.
- Convertir Folio en una red social.

## 7. Revisión post-1.0

Cada tres meses:

- Revisar issues y feedback.
- Medir uso real (local, sin telemetría; el usuario comparte si quiere).
- Reevaluar prioridades.
- Actualizar este backlog y las ADR.
- No añadir más de dos frentes grandes a la vez.
