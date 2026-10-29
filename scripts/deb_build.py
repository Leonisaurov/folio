#!/usr/bin/env python3
"""Construye y verifica un .deb sin depender de dpkg-deb.

Formato: ar(debian-binary, control.tar.gz, data.tar.gz). Permite empaquetar en
Arch y verificar el contenido en CI; si existe dpkg-deb, build-deb.sh lo prefiere.
"""
from __future__ import annotations

import gzip
import io
import os
import stat
import sys
import time
import tarfile
from pathlib import Path

AR_HEADER = b"!<arch>\n"


def _ar_member(name: str, data: bytes) -> bytes:
    header = (
        f"{name:<16}{int(time.time()):<12}{0:<6}{0:<6}{0o100644:<8o}"
        f"{len(data):<10}`\n"
    ).encode("ascii")
    assert len(header) == 60, len(header)
    payload = data + (b"\n" if len(data) % 2 else b"")
    return header + payload


def _tar_gz(files: list[tuple[Path, str]]) -> bytes:
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz", format=tarfile.GNU_FORMAT) as tar:
        for source, arcname in files:
            info = tar.gettarinfo(str(source), arcname=arcname)
            info.uid = 0
            info.gid = 0
            info.uname = "root"
            info.gname = "root"
            if source.is_file():
                with source.open("rb") as handle:
                    tar.addfile(info, handle)
            else:
                tar.addfile(info)
    return buffer.getvalue()


def build(root: Path, output: Path) -> None:
    control_dir = root / "DEBIAN"
    control = _tar_gz([(control_dir / "control", "./control")])
    files: list[tuple[Path, str]] = []
    for path in sorted(root.rglob("*")):
        if control_dir in path.parents or path == control_dir:
            continue
        arcname = "./" + str(path.relative_to(root))
        files.append((path, arcname))
    data = _tar_gz(files)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(
        AR_HEADER
        + _ar_member("debian-binary", b"2.0\n")
        + _ar_member("control.tar.gz", control)
        + _ar_member("data.tar.gz", data)
    )


def _ar_members(blob: bytes) -> dict[str, bytes]:
    assert blob.startswith(AR_HEADER), "no es un archivo ar"
    pos = len(AR_HEADER)
    members = {}
    while pos < len(blob):
        header = blob[pos : pos + 60]
        pos += 60
        if len(header) < 60:
            break
        name = header[:16].decode("ascii").strip()
        size = int(header[48:58].decode("ascii").strip())
        data = blob[pos : pos + size]
        pos += size + (size % 2)
        members[name] = data
    return members


def verify(deb: Path, extract: Path) -> None:
    members = _ar_members(deb.read_bytes())
    assert members.get("debian-binary") == b"2.0\n"
    control = tarfile.open(fileobj=io.BytesIO(members["control.tar.gz"]), mode="r:gz")
    names = control.getnames()
    assert "./control" in names or "control" in names, names
    extract.mkdir(parents=True, exist_ok=True)
    tarfile.open(fileobj=io.BytesIO(members["data.tar.gz"]), mode="r:gz").extractall(
        extract, filter="data"
    )
    required = [
        "usr/bin/folio",
        "usr/bin/folio-tools",
        "usr/share/applications/io.github.leonisauro.Folio.desktop",
        "usr/share/metainfo/io.github.leonisauro.Folio.metainfo.xml",
        "usr/share/icons/hicolor/scalable/apps/io.github.leonisauro.Folio.svg",
    ]
    for relative in required:
        path = extract / relative
        assert path.exists(), f"falta {relative}"
    assert os.access(extract / "usr/bin/folio", os.X_OK)
    mode = stat.S_IMODE((extract / "usr/bin/folio").stat().st_mode)
    assert mode & 0o111, mode
    print(f"control: {Path('DEBIAN/control').name} con {len(control.getmembers())} entrada(s)")
    print(f"archivos: {len(required)} requeridos presentes; tamaño {deb.stat().st_size} bytes")


if __name__ == "__main__":
    if sys.argv[1] == "build":
        build(Path(sys.argv[2]), Path(sys.argv[3]))
    elif sys.argv[1] == "verify":
        verify(Path(sys.argv[2]), Path(sys.argv[3]))
    else:
        raise SystemExit("uso: deb_build.py build|verify ...")
