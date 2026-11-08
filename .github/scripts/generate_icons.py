#!/usr/bin/env python3
"""Regenerate checked-in app icons from the README logo (requires rsvg-convert and ImageMagick)."""

from pathlib import Path
import struct
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "assets/icons"


def generate():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as temporary:
        work = Path(temporary)
        images = {}
        for size in (16, 24, 32, 48, 64, 128, 256, 512, 1024):
            image = work / f"{size}.png"
            subprocess.run([
                "rsvg-convert", "--width", str(size), "--height", str(size),
                "--output", str(image), str(ROOT / ".github/assets/logo.svg"),
            ], check=True)
            images[size] = image
        (OUTPUT / "pomoterra.png").write_bytes(images[256].read_bytes())
        subprocess.run([
            "convert", str(images[64]), "-depth", "8", f"rgba:{OUTPUT / 'pomoterra.rgba'}",
        ], check=True)
        subprocess.run([
            "convert", *(str(images[size]) for size in (16, 24, 32, 48, 64, 128, 256)),
            str(OUTPUT / "pomoterra.ico"),
        ], check=True)
        chunks = []
        for kind, size in ((b"icp4", 16), (b"icp5", 32), (b"icp6", 64),
                           (b"ic07", 128), (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024)):
            data = images[size].read_bytes()
            chunks.append(kind + struct.pack(">I", len(data) + 8) + data)
        body = b"".join(chunks)
        (OUTPUT / "pomoterra.icns").write_bytes(b"icns" + struct.pack(">I", len(body) + 8) + body)


if __name__ == "__main__":
    generate()
