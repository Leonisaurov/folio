#!/usr/bin/env python3
"""Valida la integridad del plan de Folio.

Comprueba:
- que existan los documentos obligatorios;
- que cada documento tenga las secciones clave;
- que las 14 etapas E0..E13 estén en la hoja de ruta;
- que no haya enlaces locales rotos;
- que la matriz de trazabilidad cubra los requisitos del MVP.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent

REQUIRED_FILES = [
    "README.md",
    "PLAN.md",
    "docs/01-vision-alcance.md",
    "docs/02-arquitectura.md",
    "docs/03-modelo-datos.md",
    "docs/04-experiencia-ui.md",
    "docs/05-configuracion.md",
    "docs/06-plugins.md",
    "docs/07-seguridad-rendimiento.md",
    "docs/08-calidad-pruebas.md",
    "docs/09-hoja-de-ruta.md",
    "docs/10-decisiones.md",
    "docs/11-preguntas-abiertas.md",
    "docs/12-backlog-post-1.0.md",
    "docs/13-primeros-pasos.md",
    "docs/14-trazabilidad.md",
    "docs/15-automatizacion.md",
    "AGENTS.md",
]

REQUIRED_SECTIONS = {
    "PLAN.md": [
        "Resumen ejecutivo",
        "Decisiones cerradas con el usuario",
        "Alcance",
        "Arquitectura de un vistazo",
        "Modelo de datos de un vistazo",
        "Hoja de ruta de 14 semanas",
        "Presupuestos de rendimiento",
        "Riesgos principales",
        "Definición de éxito",
    ],
    "docs/01-vision-alcance.md": [
        "Visión",
        "Usuario y contexto",
        "Glosario",
        "Alcance por prioridad",
        "Escenario de aceptación del MVP",
        "Métricas de éxito",
        "Supuestos",
    ],
    "docs/02-arquitectura.md": [
        "Stack y versiones",
        "Workspace de crates",
        "Flujo de una operación",
        "Almacenamiento e índice",
        "Arquitectura del editor",
        "Arquitectura del lienzo",
        "Plugins",
        "Exportación",
        "Spikes",
    ],
    "docs/03-modelo-datos.md": [
        "Estructura de la bóveda",
        "Identificadores",
        "Nota: esquema JSON",
        "Tipos de bloque",
        "Orden de notas",
        "Escritura atómica y journal",
        "Índice SQLite",
        "Cifrado",
        "Migraciones",
        "Invariantes y pruebas de datos",
    ],
    "docs/04-experiencia-ui.md": [
        "Arquitectura de información",
        "Primer arranque",
        "Libros diarios",
        "Lienzo híbrido",
        "Reordenar notas",
        "Órdenes de notas",
        "Búsqueda",
        "Accesibilidad",
        "Sistema visual",
        "Criterios de aceptación de UX",
    ],
    "docs/05-configuracion.md": [
        "Archivos y precedencia",
        "Esquema único",
        "Carga, validación y recarga",
        "Referencia de claves",
        "CLI",
        "Paridad y pruebas",
    ],
    "docs/06-plugins.md": [
        "Arquitectura",
        "Paquete de plugin",
        "Contrato WIT",
        "UI declarativa",
        "Permisos",
        "Límites y aislamiento",
        "Ciclo de vida",
        "Plugins de ejemplo",
        "SDK y desarrollo",
        "Seguridad",
        "Criterios de aceptación",
    ],
    "docs/07-seguridad-rendimiento.md": [
        "Modelo de amenazas",
        "Controles por capa",
        "Cifrado de bóveda",
        "Presupuestos de rendimiento",
        "Pruebas de rendimiento",
        "Pruebas de seguridad",
        "Criterios de aceptación",
    ],
    "docs/08-calidad-pruebas.md": [
        "Pirámide de pruebas",
        "Matriz de pruebas",
        "Pruebas de caos",
        "CI",
        "Definición de terminado",
        "Checklist de release",
    ],
    "docs/09-hoja-de-ruta.md": [
        "E0 — Fundamentos",
        "E1 — Núcleo",
        "E3 — Libros diarios",
        "E4 — Editor",
        "E5 — Lienzo",
        "E6 — Orden libre",
        "E10 — Plugins",
        "E11 — Backups",
        "E12 — Pulido",
        "E13 — Beta",
        "1.0 — Semanas 15–20",
        "Ritmo semanal",
    ],
    "docs/10-decisiones.md": [
        "ADR-001",
        "ADR-003",
        "ADR-008",
        "ADR-009",
        "ADR-012",
        "ADR-020",
    ],
    "docs/11-preguntas-abiertas.md": [
        "Preguntas ya cerradas",
        "Preguntas abiertas",
        "Supuestos explícitos",
    ],
    "docs/12-backlog-post-1.0.md": [
        "Prioridad alta",
        "Sincronización: diseño preparado",
        "Lo que no se hará",
    ],
    "docs/13-primeros-pasos.md": [
        "Antes de tocar código",
        "Crear el repositorio",
        "Primer binario: ventana GTK",
        "Checklist del día 1",
        "Lo que NO se debe hacer todavía",
    ],
    "docs/14-trazabilidad.md": [
        "Requisitos funcionales",
        "Requisitos no funcionales",
        "Requisitos de datos",
        "Requisitos de plugins",
        "Requisitos de configuración",
        "Cobertura de la visión",
    ],
    "docs/15-automatizacion.md": [
        "Regla de oro",
        "Comandos",
        "Qué aísla",
        "Estructura",
        "Artefactos y diagnóstico",
        "Trabajo desatendido",
        "Cómo escribir tests de UI",
        "CI",
        "Límites",
        "Criterios de aceptación",
    ],
    "AGENTS.md": [
        "Empieza aquí",
        "Reglas duras",
        "Comandos",
    ],
}

REQUIRED_SUPPORT_FILES = [
    ".github/workflows/ci.yml",
    "justfile",
    ".gitignore",
]

REQUIRED_EXECUTABLES = [
    "scripts/automation/with-x11.sh",
    "scripts/automation/plan.sh",
    "scripts/automation/quality.sh",
    "scripts/automation/unit.sh",
    "scripts/automation/ui.sh",
    "scripts/automation/all.sh",
    "scripts/automation/watch.sh",
    "scripts/automation/keep-awake.sh",
    "scripts/automation/nightly.sh",
]

LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")


def fail(message: str) -> None:
    print(f"FALLO: {message}")
    errors.append(message)


errors: list[str] = []


def main() -> int:
    print(f"Validando plan en {ROOT}\n")

    # 1. Archivos obligatorios
    for rel in REQUIRED_FILES:
        path = ROOT / rel
        if not path.is_file():
            fail(f"falta el archivo {rel}")
        else:
            print(f"  ok  {rel} ({len(path.read_text(encoding='utf-8').splitlines())} líneas)")

    # 2. Secciones obligatorias
    print("\nSecciones clave:")
    for rel, sections in REQUIRED_SECTIONS.items():
        path = ROOT / rel
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        for section in sections:
            if section not in text:
                fail(f"{rel}: falta la sección «{section}»")
            else:
                print(f"  ok  {rel} :: {section}")

    # 3. Etapas E0..E13 y calendario
    roadmap = (ROOT / "docs/09-hoja-de-ruta.md").read_text(encoding="utf-8")
    for n in range(14):
        marker = f"## E{n} —"
        if marker not in roadmap:
            fail(f"hoja de ruta: falta la etapa E{n}")
    print("\nEtapas E0..E13: ok" if all(f"## E{n} —" in roadmap for n in range(14)) else "\nEtapas: revisar")

    durations = [int(x) for x in re.findall(r"^\| Días \| (\d+) ", roadmap, re.M)]
    if len(durations) != 14:
        fail(f"hoja de ruta: se esperaban 14 duraciones, hay {len(durations)}")
    elif sum(durations) != 70:
        fail(f"hoja de ruta: las duraciones suman {sum(durations)} días, no 70")
    else:
        print(f"Calendario: 14 etapas, {sum(durations)} días = 14 semanas")

    if "## 1.0 — Semanas 15–20" not in roadmap:
        fail("hoja de ruta: falta el bloque de 1.0 (semanas 15–20)")
    else:
        print("1.0: bloque de semanas 15–20 presente")

    # 4. Bloques de código equilibrados
    for md in sorted(ROOT.rglob("*.md")):
        text = md.read_text(encoding="utf-8")
        fences = sum(1 for line in text.splitlines() if line.strip().startswith("```"))
        if fences % 2 != 0:
            fail(f"{md.relative_to(ROOT)}: bloques de código sin cerrar ({fences} fences)")

    # 4.1 Archivos de soporte
    for rel in REQUIRED_SUPPORT_FILES:
        path = ROOT / rel
        if not path.is_file():
            fail(f"falta el archivo de soporte {rel}")
        else:
            print(f"  ok  soporte {rel}")

    # 4.2 Scripts ejecutables
    for rel in REQUIRED_EXECUTABLES:
        path = ROOT / rel
        if not path.is_file():
            fail(f"falta el script {rel}")
        elif not path.stat().st_mode & 0o111:
            fail(f"el script {rel} no es ejecutable")
        else:
            print(f"  ok  ejecutable {rel}")

    # 5. Enlaces locales
    print("\nEnlaces locales:")
    broken = 0
    for md in sorted(ROOT.rglob("*.md")):
        text = md.read_text(encoding="utf-8")
        for target in LINK_RE.findall(text):
            target = target.strip()
            if target.startswith(("http://", "https://", "mailto:", "#")):
                continue
            target = target.split("#", 1)[0].strip()
            if not target:
                continue
            target = unquote(target)
            resolved = (md.parent / target).resolve()
            try:
                resolved.relative_to(ROOT.resolve())
            except ValueError:
                fail(f"{md.relative_to(ROOT)}: enlace fuera del repo: {target}")
                broken += 1
                continue
            if not resolved.exists():
                fail(f"{md.relative_to(ROOT)}: enlace roto: {target}")
                broken += 1
    if broken == 0:
        print("  ok  todos los enlaces locales resuelven")

    # 6. Trazabilidad mínima
    trace = (ROOT / "docs/14-trazabilidad.md").read_text(encoding="utf-8")
    required_ids = ["RF-01", "RF-10", "RF-12", "RF-23", "RF-31", "RF-33", "RF-34", "RF-36", "RF-42", "RF-49", "RNF-21", "RNF-22"]
    missing = [rid for rid in required_ids if rid not in trace]
    for rid in missing:
        fail(f"trazabilidad: falta {rid}")
    if not missing:
        print("\nTrazabilidad: requisitos clave cubiertos")

    # 7. Señales de alcance
    all_text = "\n".join(
        p.read_text(encoding="utf-8") for p in sorted(ROOT.rglob("*.md"))
    )
    for term in ["spring-load", "Recomponer", "WASM", "SQLite", "TOML", "cifrada"]:
        if term.lower() not in all_text.lower():
            fail(f"el plan no menciona el término clave «{term}»")

    print()
    if errors:
        print(f"Resultado: {len(errors)} fallo(s)")
        for e in errors:
            print(f"  - {e}")
        return 1
    print("Resultado: plan válido ✅")
    return 0


if __name__ == "__main__":
    sys.exit(main())
